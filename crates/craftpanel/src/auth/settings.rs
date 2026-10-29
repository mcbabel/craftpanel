use sqlx::SqlitePool;

use super::error::{Failure, Result};
use super::limits;
use crate::model::{CpuMode, PanelSettings, Timestamp, UserLimits, UserPorts};

const MAX_BACKUPS: u32 = 50;

#[derive(sqlx::FromRow)]
struct Row {
    public_address: Option<String>,
    default_memory_mib: u32,
    default_cpu_mode: CpuMode,
    default_cpu_cores: f64,
    default_pids_max: u32,
    default_disk_mib: u32,
    max_upload_bytes: i64,
    max_backups_per_server: u32,
    external_services_enabled: bool,
    max_concurrent_operations: u32,
    stop_grace_seconds: u32,
    registration_enabled: bool,
    registration_requires_approval: bool,
    java_auto_install: bool,
    user_ports: UserPorts,
    network_needs_playit: bool,
}

pub async fn load(pool: &SqlitePool) -> sqlx::Result<PanelSettings> {
    let row = sqlx::query_as::<_, Row>(
        "SELECT public_address, default_memory_mib, \
         default_cpu_mode, default_cpu_cores, default_pids_max, default_disk_mib, \
         max_upload_bytes, max_backups_per_server, external_services_enabled, \
         max_concurrent_operations, stop_grace_seconds, registration_enabled, \
         registration_requires_approval, java_auto_install, user_ports, network_needs_playit \
         FROM panel_settings WHERE id = 1",
    )
    .fetch_one(pool)
    .await?;

    Ok(PanelSettings {
        public_address: row.public_address,
        default_limits: UserLimits {
            memory_mib: row.default_memory_mib,
            cpu_mode: row.default_cpu_mode,
            cpu_cores: row.default_cpu_cores,
            pids_max: row.default_pids_max,
            disk_mib: row.default_disk_mib,
        },
        max_upload_bytes: row.max_upload_bytes.max(0) as u64,
        max_backups_per_server: row.max_backups_per_server,
        external_services_enabled: row.external_services_enabled,
        max_concurrent_operations: row.max_concurrent_operations,
        stop_grace_seconds: row.stop_grace_seconds,
        registration_enabled: row.registration_enabled,
        registration_requires_approval: row.registration_requires_approval,
        java_auto_install: row.java_auto_install,
        user_ports: row.user_ports,
        network_needs_playit: row.network_needs_playit,
    })
}

pub async fn save(pool: &SqlitePool, settings: &PanelSettings) -> Result<()> {
    check(settings)?;

    sqlx::query(
        "UPDATE panel_settings SET public_address = ?, \
         default_memory_mib = ?, default_cpu_mode = ?, default_cpu_cores = ?, \
         default_pids_max = ?, default_disk_mib = ?, max_upload_bytes = ?, \
         max_backups_per_server = ?, \
         external_services_enabled = ?, max_concurrent_operations = ?, stop_grace_seconds = ?, \
         registration_enabled = ?, registration_requires_approval = ?, \
         java_auto_install = ?, user_ports = ?, network_needs_playit = ?, updated_at = ? WHERE id = 1",
    )
    .bind(settings.public_address.as_deref())
    .bind(settings.default_limits.memory_mib)
    .bind(settings.default_limits.cpu_mode)
    .bind(settings.default_limits.cpu_cores)
    .bind(settings.default_limits.pids_max)
    .bind(settings.default_limits.disk_mib)
    .bind(settings.max_upload_bytes as i64)
    .bind(settings.max_backups_per_server)
    .bind(settings.external_services_enabled)
    .bind(settings.max_concurrent_operations)
    .bind(settings.stop_grace_seconds)
    .bind(settings.registration_enabled)
    .bind(settings.registration_requires_approval)
    .bind(settings.java_auto_install)
    .bind(settings.user_ports)
    .bind(settings.network_needs_playit)
    .bind(Timestamp::now())
    .execute(pool)
    .await?;

    Ok(())
}

fn check(settings: &PanelSettings) -> Result<()> {
    limits::check(&settings.default_limits)?;

    if settings.max_upload_bytes == 0 {
        return Err(Failure::invalid_request("max_upload_bytes is above zero"));
    }
    if !(1..=MAX_BACKUPS).contains(&settings.max_backups_per_server) {
        return Err(Failure::invalid_request(format!(
            "max_backups_per_server is between 1 and {MAX_BACKUPS}"
        )));
    }
    if settings.max_concurrent_operations == 0 {
        return Err(Failure::invalid_request("max_concurrent_operations is at least one"));
    }
    if settings.stop_grace_seconds == 0 {
        return Err(Failure::invalid_request("stop_grace_seconds is above zero"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::harness::test_pool;

    fn changed(base: &PanelSettings) -> PanelSettings {
        PanelSettings {
            public_address: Some("minecraft.example".to_owned()),
            max_upload_bytes: 1024,
            max_backups_per_server: 5,
            external_services_enabled: false,
            max_concurrent_operations: 4,
            stop_grace_seconds: 30,
            java_auto_install: false,
            user_ports: UserPorts::Locked,
            network_needs_playit: true,
            ..base.clone()
        }
    }

    #[tokio::test]
    async fn the_defaults_of_0002_are_what_the_contract_names() {
        let pool = test_pool().await;
        let settings = load(&pool).await.unwrap();

        assert_eq!(settings.public_address, None, "12.10: the machine cannot know it");
        assert!(settings.external_services_enabled, "17.8: on by default");
        assert_eq!(settings.max_backups_per_server, 10);
        assert_eq!(settings.max_upload_bytes, 4 * 1024 * 1024 * 1024);
        assert_eq!(settings.default_limits.disk_mib, 51200, "0007: fifty gibibytes");
        assert!(settings.java_auto_install, "0015: the only switch that starts open");
    }

    #[tokio::test]
    async fn what_is_written_comes_back() {
        let pool = test_pool().await;
        let wanted = changed(&load(&pool).await.unwrap());

        save(&pool, &wanted).await.unwrap();
        assert_eq!(load(&pool).await.unwrap(), wanted);
    }

    #[tokio::test]
    async fn the_remaining_bounds_are_each_refused_on_their_own() {
        let pool = test_pool().await;
        let good = load(&pool).await.unwrap();

        let cases: Vec<(&str, PanelSettings)> = vec![
            ("no upload", PanelSettings { max_upload_bytes: 0, ..good.clone() }),
            ("too many backups", PanelSettings { max_backups_per_server: 51, ..good.clone() }),
            ("no backups", PanelSettings { max_backups_per_server: 0, ..good.clone() }),
            ("no operations", PanelSettings { max_concurrent_operations: 0, ..good.clone() }),
            ("no grace", PanelSettings { stop_grace_seconds: 0, ..good.clone() }),
            (
                "tiny default limit",
                PanelSettings {
                    default_limits: UserLimits { memory_mib: 8, ..good.default_limits },
                    ..good.clone()
                },
            ),
            (
                "tiny default disk",
                PanelSettings {
                    default_limits: UserLimits { disk_mib: 1023, ..good.default_limits },
                    ..good.clone()
                },
            ),
        ];

        for (what, wanted) in cases {
            assert_eq!(
                save(&pool, &wanted).await.unwrap_err().code(),
                "invalid_request",
                "{what} should be refused"
            );
        }
        assert_eq!(load(&pool).await.unwrap(), good, "nothing of that was written");
    }

    #[tokio::test]
    async fn the_id_column_keeps_it_at_one_row() {
        let pool = test_pool().await;
        let second = sqlx::query("INSERT INTO panel_settings (id, port_pool_from, port_pool_to, \
             default_memory_mib, default_cpu_mode, default_cpu_cores, default_pids_max, \
             max_upload_bytes, max_backups_per_server, max_concurrent_operations, \
             stop_grace_seconds, updated_at) VALUES (2, 25565, 25700, 4096, 'cap', 2.0, 512, \
             1024, 10, 2, 60, ?)")
            .bind(Timestamp::now())
            .execute(&pool)
            .await;
        assert!(second.is_err(), "a second row would make 'the settings' ambiguous");
    }
}
