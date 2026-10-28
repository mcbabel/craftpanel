use std::sync::Arc;
use std::time::Duration;

use crate::auth::error::{Failure, Result};
use crate::model::{BackupSchedule, BackupScheduleStatus, Id, Timestamp};

use super::store::{self, Schedule};
use super::Backups;

const TICK: Duration = Duration::from_secs(60);

pub const MIN_INTERVAL: u32 = 1;
pub const MAX_INTERVAL: u32 = 168;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateBackupScheduleRequest {
    pub automatic_slots: Option<u32>,
    pub interval_hours: Option<u32>,
    pub hour_utc: Option<u8>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupScheduleUpdate {
    pub applied: bool,
    pub backup_ids: Vec<Id>,
    pub schedule: BackupSchedule,
}

impl UpdateBackupScheduleRequest {
    pub fn check(&self, quota: u32) -> Result<()> {
        let refuse = |message: String| Err(Failure::bad_request("invalid_schedule", message));
        if let Some(hours) = self.interval_hours {
            if !(MIN_INTERVAL..=MAX_INTERVAL).contains(&hours) {
                return refuse(format!(
                    "interval_hours is {hours} and has to be between {MIN_INTERVAL} and \
                     {MAX_INTERVAL}"
                ));
            }
        }
        if let Some(hour) = self.hour_utc {
            if hour > 23 {
                return refuse(format!("hour_utc is {hour} and has to be between 0 and 23"));
            }
        }
        if let Some(slots) = self.automatic_slots {
            if slots > quota {
                return refuse(format!(
                    "automatic_slots is {slots} and cannot be more than the quota of {quota}"
                ));
            }
        }
        Ok(())
    }

    fn apply_to(&self, current: &Schedule, now: Timestamp) -> Schedule {
        let slots = self
            .automatic_slots
            .unwrap_or(if current.enabled { current.keep_last } else { 0 });
        let enabled = slots > 0;
        let interval_hours = self.interval_hours.unwrap_or(current.interval_hours);
        let hour_utc = self.hour_utc.unwrap_or(current.hour_utc);
        let timing_kept = current.enabled
            && interval_hours == current.interval_hours
            && hour_utc == current.hour_utc;
        let next_run_at = match (enabled, timing_kept, current.next_run_at) {
            (false, _, _) => None,
            (true, true, Some(next)) => Some(next),
            (true, _, _) => Some(next_after(now, interval_hours, hour_utc)),
        };
        Schedule {
            enabled,
            interval_hours,
            hour_utc,
            keep_last: if enabled { slots } else { current.keep_last },
            next_run_at,
            ..current.clone()
        }
    }
}

pub fn next_after(from: Timestamp, interval_hours: u32, hour_utc: u8) -> Timestamp {
    let from = from.as_datetime();
    if interval_hours % 24 == 0 {
        let days = (interval_hours / 24).max(1) as i64;
        let at = time::Time::from_hms(hour_utc.min(23), 0, 0).expect("an hour below 24");
        let mut candidate = from.replace_time(at);
        while candidate <= from {
            candidate += time::Duration::days(days);
        }
        Timestamp::at(candidate)
    } else {
        Timestamp::at(from + time::Duration::hours(interval_hours as i64))
    }
}

impl Backups {
    pub async fn schedule(&self, server: Id) -> Result<BackupSchedule> {
        let stored = store::schedule(self.pool(), server).await?;
        self.describe(server, stored).await
    }

    async fn describe(&self, server: Id, stored: Schedule) -> Result<BackupSchedule> {
        let limit = store::quota(self.pool()).await?;
        let automatic_slots = stored.automatic_slots(limit);
        Ok(BackupSchedule {
            limit,
            automatic_slots,
            manual_slots: limit - automatic_slots,
            automatic_used: store::count_of_kind(self.pool(), server, true).await?,
            manual_used: store::count_of_kind(self.pool(), server, false).await?,
            interval_hours: stored.interval_hours,
            hour_utc: stored.hour_utc,
            next_run_at: stored.next_run_at,
            last_run_at: stored.last_run_at,
            last_status: stored.last_status,
            last_error: stored.last_error,
        })
    }

    pub async fn manual_slots(&self, server: Id) -> Result<u32> {
        let quota = store::quota(self.pool()).await?;
        let stored = store::schedule(self.pool(), server).await?;
        Ok(quota - stored.automatic_slots(quota))
    }

    /// Fewer automatic slots than there are automatic backups rotates the oldest ones away,
    /// which is only done once the caller has seen which ones and said yes.
    pub async fn update_schedule(
        self: &Arc<Self>,
        server: Id,
        wanted: UpdateBackupScheduleRequest,
        acknowledge_rotation: bool,
    ) -> Result<BackupScheduleUpdate> {
        let quota = store::quota(self.pool()).await?;
        wanted.check(quota)?;
        let current = store::schedule(self.pool(), server).await?;
        let next = wanted.apply_to(&current, Timestamp::now());
        let slots = next.automatic_slots(quota);
        if next.enabled && !current.enabled {
            self.drive().guard_reachable(server).await?;
        }

        let doomed = store::automatic_over(self.pool(), server, slots).await?;
        if !doomed.is_empty() && !acknowledge_rotation {
            return Ok(BackupScheduleUpdate {
                applied: false,
                backup_ids: doomed,
                schedule: self.describe(server, current).await?,
            });
        }

        let saved = store::save_schedule(self.pool(), server, &next).await?;
        let rotated = self.prune(server, slots).await?;
        Ok(BackupScheduleUpdate {
            applied: true,
            backup_ids: rotated,
            schedule: self.describe(server, saved).await?,
        })
    }

    pub async fn tick(self: &Arc<Self>, now: Timestamp) -> Vec<(Id, BackupScheduleStatus)> {
        let due = match store::due(self.pool(), now).await {
            Ok(due) => due,
            Err(err) => {
                tracing::error!("the backup schedule could not be read: {}", err);
                return Vec::new();
            }
        };

        let mut done = Vec::new();
        for server in due {
            match self.run_scheduled(server, now).await {
                Ok(status) => done.push((server, status)),
                Err(err) => tracing::error!(%server, "the scheduled backup failed: {err}"),
            }
        }
        done
    }

    async fn run_scheduled(
        self: &Arc<Self>,
        server: Id,
        now: Timestamp,
    ) -> Result<BackupScheduleStatus> {
        let schedule = store::schedule(self.pool(), server).await?;
        let next = next_after(now, schedule.interval_hours, schedule.hour_utc);

        store::reserve(self.pool(), server, next).await?;

        let settle = |status, error: Option<String>| {
            let pool = self.pool().clone();
            async move {
                store::record_run(&pool, server, now, status, error.as_deref(), Some(next)).await
            }
        };

        let quota = store::quota(self.pool()).await?;
        let manual = store::count_of_kind(self.pool(), server, false).await?;
        let room = schedule.automatic_slots(quota).min(quota.saturating_sub(manual));
        if room == 0 {
            settle(BackupScheduleStatus::SkippedLimit, None).await?;
            return Ok(BackupScheduleStatus::SkippedLimit);
        }
        if !self.changed_since_last_automatic(server).await? {
            settle(BackupScheduleStatus::SkippedUnchanged, None).await?;
            return Ok(BackupScheduleStatus::SkippedUnchanged);
        }

        let name = format!("Automatic backup {now}");
        let queued = self.create(server, &name, None, true).await?;
        self.run(queued.operation).await;

        let outcome = store::newest_run(self.pool(), queued.backup).await?;
        let (status, error) = match outcome.map(|run| run.state) {
            Some(crate::model::BackupOperationState::Completed) => {
                self.dismiss(server, queued.operation).await;
                (BackupScheduleStatus::Completed, None)
            }
            Some(crate::model::BackupOperationState::TimedOut) => {
                (BackupScheduleStatus::TimedOut, Some("the run timed out".to_owned()))
            }
            _ => (BackupScheduleStatus::Failed, Some("the run did not finish".to_owned())),
        };
        settle(status, error).await?;

        if status == BackupScheduleStatus::Completed {
            self.prune(server, room).await?;
        }
        Ok(status)
    }

    async fn changed_since_last_automatic(&self, server: Id) -> Result<bool> {
        let Some(finished) = store::newest_automatic_finish(self.pool(), server).await? else {
            return Ok(true);
        };
        let directory = self.server_dir(server).await?;
        let plan = tokio::task::spawn_blocking(move || super::archive::survey(&directory))
            .await
            .map_err(|err| Failure::internal(anyhow::anyhow!("{err}")))?
            .map_err(Failure::internal)?;
        let Some(newest) = plan.newest else {
            return Ok(false);
        };
        Ok(Timestamp::at(newest.into()) > finished)
    }

    pub async fn prune(self: &Arc<Self>, server: Id, keep: u32) -> Result<Vec<Id>> {
        let doomed = store::automatic_over(self.pool(), server, keep).await?;
        let mut gone = Vec::new();
        for backup in doomed {
            if store::is_busy(self.pool(), backup).await? {
                continue;
            }
            self.forget(server, backup).await?;
            gone.push(backup);
        }
        if !gone.is_empty() {
            self.announce(server).await;
        }
        Ok(gone)
    }

    pub fn spawn_scheduler(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let backups = Arc::clone(self);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(TICK);
            loop {
                tick.tick().await;
                backups.tick(Timestamp::now()).await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backups::testing::FakeServer;
    use crate::model::BackupStatus;

    fn at(text: &str) -> Timestamp {
        text.parse().expect("a timestamp")
    }

    #[test]
    fn a_daily_schedule_keeps_its_hour_and_an_odd_interval_just_counts_hours() {
        assert_eq!(
            next_after(at("2026-08-12T02:00:00Z"), 24, 4),
            at("2026-08-12T04:00:00Z"),
            "later today"
        );
        assert_eq!(
            next_after(at("2026-08-12T04:00:00Z"), 24, 4),
            at("2026-08-13T04:00:00Z"),
            "the hour has just gone, so tomorrow"
        );
        assert_eq!(
            next_after(at("2026-08-12T05:00:00Z"), 48, 4),
            at("2026-08-14T04:00:00Z"),
            "two whole days, still at the hour"
        );
        assert_eq!(
            next_after(at("2026-08-12T05:30:00Z"), 6, 4),
            at("2026-08-12T11:30:00Z"),
            "10.10: hour_utc is only read when the interval is whole days"
        );
    }

    #[test]
    fn the_three_limits_of_10_10_name_the_field_they_are_about() {
        let good = |interval, hour, slots| UpdateBackupScheduleRequest {
            automatic_slots: Some(slots),
            interval_hours: Some(interval),
            hour_utc: Some(hour),
        };
        assert!(good(24, 4, 5).check(10).is_ok());
        assert!(good(24, 4, 0).check(10).is_ok(), "no automatic slots is how it is switched off");
        assert!(UpdateBackupScheduleRequest::default().check(10).is_ok(), "nothing to change");

        for (request, field) in [
            (good(0, 4, 5), "interval_hours"),
            (good(169, 4, 5), "interval_hours"),
            (good(24, 24, 5), "hour_utc"),
            (good(24, 4, 11), "automatic_slots"),
        ] {
            let refused = request.check(10).expect_err("out of bounds");
            assert_eq!(refused.code(), "invalid_schedule");
            assert!(format!("{refused}").contains(field), "{refused} should name {field}");
        }
    }

    #[test]
    fn the_next_run_moves_only_when_the_timing_does() {
        let now = at("2026-08-12T02:00:00Z");
        let running = Schedule {
            enabled: true,
            interval_hours: 24,
            hour_utc: 4,
            keep_last: 5,
            next_run_at: Some(at("2026-08-12T04:00:00Z")),
            last_run_at: None,
            last_status: None,
            last_error: None,
        };

        let more_slots =
            UpdateBackupScheduleRequest { automatic_slots: Some(8), ..Default::default() };
        let kept = more_slots.apply_to(&running, at("2026-08-12T03:59:00Z"));
        assert_eq!(kept.keep_last, 8);
        assert_eq!(kept.next_run_at, running.next_run_at);

        let shorter = UpdateBackupScheduleRequest { interval_hours: Some(6), ..Default::default() };
        let moved = shorter.apply_to(&running, now);
        assert_eq!(moved.next_run_at, Some(at("2026-08-12T08:00:00Z")));

        let off = UpdateBackupScheduleRequest { automatic_slots: Some(0), ..Default::default() };
        let stopped = off.apply_to(&running, now);
        assert!(!stopped.enabled);
        assert_eq!(stopped.next_run_at, None);
        assert_eq!(stopped.keep_last, 5, "the column keeps a number it may hold");
        assert_eq!(stopped.automatic_slots(10), 0);
    }

    #[tokio::test]
    async fn a_due_schedule_makes_an_automatic_backup_and_does_not_prompt_about_it() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        enable(&game, 24, 5).await;

        let now = Timestamp::now();
        let done = game.backups.tick(now).await;
        assert_eq!(done, vec![(game.server, BackupScheduleStatus::Completed)]);

        let list = game.backups.list(game.server).await.expect("a list");
        assert_eq!(list.backups.len(), 1);
        let made = &list.backups[0];
        assert!(made.automated, "the pill 'Auto' has to find something");
        assert_eq!(made.status, BackupStatus::Done);
        assert!(
            !made.history[0].should_prompt,
            "10.1: a successful automatic backup must not greet its owner every morning"
        );

        let schedule = game.backups.schedule(game.server).await.expect("a schedule");
        assert_eq!(schedule.last_status, Some(BackupScheduleStatus::Completed));
        assert!(schedule.next_run_at.expect("a next time") > now);
    }

    #[tokio::test]
    async fn a_server_that_has_not_changed_is_skipped_and_a_changed_one_is_not() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        enable(&game, 1, 5).await;

        assert_eq!(
            game.backups.tick(Timestamp::now()).await,
            vec![(game.server, BackupScheduleStatus::Completed)]
        );

        due_now(&game).await;
        assert_eq!(
            game.backups.tick(Timestamp::now()).await,
            vec![(game.server, BackupScheduleStatus::SkippedUnchanged)],
            "nothing moved, so a second copy of the same bytes is waste"
        );
        assert_eq!(game.backup_rows().await, 1);

        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        game.file("world/region/r.0.0.mca", b"new chunks");
        due_now(&game).await;
        assert_eq!(
            game.backups.tick(Timestamp::now()).await,
            vec![(game.server, BackupScheduleStatus::Completed)]
        );
        assert_eq!(game.backup_rows().await, 2);
    }

    #[tokio::test]
    async fn keep_last_takes_automatic_backups_and_never_a_hand_made_one() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        let by_hand = game.a_finished_backup("mine").await;

        for index in 0..3 {
            let queued = game
                .backups
                .create(game.server, &format!("auto {index}"), None, true)
                .await
                .expect("an automatic backup");
            game.backups.run(queued.operation).await;
        }
        assert_eq!(game.backup_rows().await, 4, "one by hand and three automatic ones");

        let gone = game.backups.prune(game.server, 1).await.expect("a prune");
        assert_eq!(gone.len(), 2);

        let left = game.backups.list(game.server).await.expect("a list");
        assert_eq!(left.backups.len(), 2);
        assert!(
            left.backups.iter().any(|backup| backup.id == by_hand),
            "10.12: a rule never takes away what somebody made by hand"
        );
        assert_eq!(left.backups.iter().filter(|backup| backup.automated).count(), 1);
    }

    #[tokio::test]
    async fn a_failed_automatic_backup_does_prompt_and_says_so_in_the_schedule() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        enable(&game, 24, 5).await;

        std::fs::create_dir_all(game.backups.root()).expect("the backup root");
        std::fs::write(game.backups.dir_of(game.server), b"in the way").expect("an obstacle");

        assert_eq!(
            game.backups.tick(Timestamp::now()).await,
            vec![(game.server, BackupScheduleStatus::Failed)]
        );

        let list = game.backups.list(game.server).await.expect("a list");
        assert_eq!(list.backups[0].status, BackupStatus::Error);
        assert!(
            list.backups[0].history[0].should_prompt,
            "10.1: that the schedule is not running has to reach somebody"
        );

        let schedule = game.backups.schedule(game.server).await.expect("a schedule");
        assert_eq!(schedule.last_status, Some(BackupScheduleStatus::Failed));
        assert!(schedule.last_error.is_some());
    }

    #[tokio::test]
    async fn a_tick_that_cannot_do_its_work_still_gives_up_its_slot() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        enable(&game, 24, 5).await;
        game.backups.create(game.server, "by hand", None, false).await.expect("a queued backup");

        let before = Timestamp::now();
        assert!(game.backups.tick(before).await.is_empty(), "nothing was made");

        let schedule = game.backups.schedule(game.server).await.expect("a schedule");
        assert!(schedule.next_run_at.expect("a next time") > before);
        assert_eq!(game.backup_rows().await, 1, "and no half made row was left behind");
    }

    #[tokio::test]
    async fn a_full_quota_is_recorded_and_nothing_is_deleted_to_make_room() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        game.a_finished_backup("mine").await;
        game.set_quota(1).await;
        enable(&game, 1, 0).await;

        assert_eq!(
            game.backups.tick(Timestamp::now()).await,
            vec![(game.server, BackupScheduleStatus::SkippedLimit)]
        );
        assert_eq!(game.backup_rows().await, 1);
    }

    #[tokio::test]
    async fn manual_backups_only_get_the_slots_the_automatic_ones_leave() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        game.set_quota(3).await;
        enable(&game, 24, 2).await;

        let schedule = game.backups.schedule(game.server).await.expect("a schedule");
        assert_eq!((schedule.automatic_slots, schedule.manual_slots), (2, 1));

        let made = game.backups.request(game.server, "mine", game.owner).await.expect("one fits");
        game.await_backup(made.id).await;
        let refused = game
            .backups
            .request(game.server, "and another", game.owner)
            .await
            .expect_err("the one manual slot is taken");
        assert_eq!(refused.code(), "backup_limit_reached");
        assert!(format!("{refused}").contains("manual"), "{refused}");
    }

    #[tokio::test]
    async fn full_automatic_slots_rotate_the_oldest_away_instead_of_stopping() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        game.set_quota(3).await;
        let by_hand = game.a_finished_backup("mine").await;
        enable(&game, 1, 2).await;

        let mut made = Vec::new();
        for round in 0..3 {
            if round > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
                game.file(&format!("world/region/r.{round}.0.mca"), b"new chunks");
                due_now(&game).await;
            }
            assert_eq!(
                game.backups.tick(Timestamp::now()).await,
                vec![(game.server, BackupScheduleStatus::Completed)],
                "round {round}"
            );
            let list = game.backups.list(game.server).await.expect("a list");
            made.push(list.backups.iter().find(|backup| backup.automated).expect("one").id);
        }

        let left = game.backups.list(game.server).await.expect("a list");
        let ids: Vec<Id> = left.backups.iter().map(|backup| backup.id).collect();
        assert_eq!(ids.len(), 3, "two automatic ones and the one by hand");
        assert!(ids.contains(&by_hand));
        assert!(!ids.contains(&made[0]), "the oldest automatic one made room");
        assert!(ids.contains(&made[1]) && ids.contains(&made[2]));
    }

    #[tokio::test]
    async fn fewer_automatic_slots_ask_before_anything_is_deleted() {
        let game = FakeServer::stopped().await;
        game.file("world/level.dat", b"a world");
        enable(&game, 24, 3).await;
        let mut automatic = Vec::new();
        for index in 0..3 {
            let queued = game
                .backups
                .create(game.server, &format!("auto {index}"), None, true)
                .await
                .expect("an automatic backup");
            game.backups.run(queued.operation).await;
            automatic.push(queued.backup);
        }

        let off = UpdateBackupScheduleRequest { automatic_slots: Some(1), ..Default::default() };
        let asked =
            game.backups.update_schedule(game.server, off.clone(), false).await.expect("an answer");
        assert!(!asked.applied);
        assert_eq!(asked.backup_ids, vec![automatic[1], automatic[0]]);
        assert_eq!(game.backup_rows().await, 3);
        assert_eq!(asked.schedule.automatic_slots, 3);

        let told = game.backups.update_schedule(game.server, off, true).await.expect("an answer");
        assert!(told.applied);
        assert_eq!(told.backup_ids, vec![automatic[1], automatic[0]]);
        assert_eq!(game.backup_rows().await, 1);
        assert_eq!(told.schedule.automatic_slots, 1);
    }

    #[tokio::test]
    async fn automatic_backups_are_not_switched_on_while_there_is_no_drive_to_put_them_in() {
        let game = FakeServer::stopped().await;
        crate::drive::harness::with_credentials(&game.drive).await;
        game.drive
            .save(
                Some("1234.apps.googleusercontent.com".to_owned()),
                crate::drive::SecretChange::Keep,
                crate::model::BackupTargetPolicy::DriveOnly,
                "craftpanel-backups".to_owned(),
                Timestamp::now(),
            )
            .await
            .expect("drive only");

        let on = UpdateBackupScheduleRequest { automatic_slots: Some(3), ..Default::default() };
        let refused = game
            .backups
            .update_schedule(game.server, on, true)
            .await
            .expect_err("every run would fail");
        assert_eq!(refused.code(), "drive_not_connected");
        let schedule = game.backups.schedule(game.server).await.expect("a schedule");
        assert_eq!(schedule.automatic_slots, 0);
    }

    async fn enable(game: &FakeServer, interval_hours: u32, automatic_slots: u32) {
        let wanted = UpdateBackupScheduleRequest {
            automatic_slots: Some(automatic_slots.max(1)),
            interval_hours: Some(interval_hours),
            hour_utc: Some(4),
        };
        let update =
            game.backups.update_schedule(game.server, wanted, true).await.expect("a schedule");
        assert!(update.applied);
        due_now(game).await;
    }

    async fn due_now(game: &FakeServer) {
        sqlx::query("UPDATE backup_schedules SET next_run_at = '2020-01-01T00:00:00Z'")
            .execute(game.pool())
            .await
            .expect("a due schedule");
    }
}
