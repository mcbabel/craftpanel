use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::http::StatusCode;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;

use crate::auth::error::{Failure, Result};
use crate::model::{Id, OperationErrorStep};
use crate::ops::{Operations, Step};

use super::archive::{
    clone_root, drive, failed, not_handed_back, share, trouble, wait_for_a_turn, Halt,
};
use super::jail::{Dir, Kind, Part, Root};
use super::path::{self, RelPath, MAX_DEPTH};
use super::{Workspace, WORK_DIR};

pub const MAX_PACK_BYTES: u64 = 20 * 1024 * 1024 * 1024;
pub const MAX_PACK_ENTRIES: u64 = 200_000;
pub const MAX_INCLUDED: usize = 5_000;
const MAX_RENAMES: u32 = 100;
const CHUNK: usize = 1024 * 1024;
const REPORT_EVERY: u64 = 8 * 1024 * 1024;

const ALREADY_PACKED: [&str; 14] = [
    ".zip", ".jar", ".gz", ".tgz", ".xz", ".zst", ".7z", ".bz2", ".mrpack", ".png", ".jpg",
    ".jpeg", ".ogg", ".webp",
];

#[derive(Debug, Clone, Deserialize)]
pub struct ZipRequest {
    pub parent: String,
    pub include: Vec<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub on_conflict: ZipConflict,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZipConflict {
    #[default]
    Fail,
    Rename,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub parent: RelPath,
    pub include: Vec<String>,
    pub target: String,
    pub entries: u64,
    pub bytes: u64,
}

impl Plan {
    pub fn archive(&self) -> RelPath {
        self.parent.with_name(&self.target)
    }
}

pub fn plan(root: &Root, request: &ZipRequest) -> Result<Plan> {
    let parent = RelPath::parse(&request.parent)?;
    if request.include.is_empty() {
        return Err(Failure::bad_request("nothing_to_pack", "no files were named"));
    }
    if request.include.len() > MAX_INCLUDED {
        return Err(too_large());
    }

    let dir = root.dir(&parent).map_err(|err| match err.raw_os_error() {
        Some(libc::ENOTDIR) => Failure::bad_request("not_a_directory", "this is not a directory"),
        _ => super::fault(&err, "not_found"),
    })?;

    let mut include: Vec<String> = Vec::with_capacity(request.include.len());
    for name in &request.include {
        path::check_name(name)?;
        if include.contains(name) {
            continue;
        }
        if parent.is_root() && name == WORK_DIR {
            return Err(Failure::not_found("not_found", "no such path"));
        }
        match dir.meta(name.as_bytes()).map_err(|err| super::fault(&err, "not_found"))?.kind {
            Kind::File | Kind::Directory => include.push(name.clone()),
            _ => {
                return Err(Failure::bad_request(
                    "not_a_regular_file",
                    "only files and directories can be packed",
                ))
            }
        }
    }

    let wanted = match request.target.as_deref().map(str::trim) {
        Some(given) if !given.is_empty() => given.to_owned(),
        _ => default_name(&include),
    };
    path::check_name(&wanted)?;
    if wanted.len() <= 4 || !wanted.to_ascii_lowercase().ends_with(".zip") {
        return Err(Failure::bad_request("invalid_name", "the archive name has to end in .zip"));
    }
    let target = free_name(&dir, &wanted, request.on_conflict)?;

    let mut found = Tally::default();
    for name in &include {
        tally(&dir, name.as_bytes(), 0, &mut found);
    }
    if found.over_the_ceiling() {
        return Err(too_large());
    }

    Ok(Plan { parent, include, target, entries: found.entries, bytes: found.bytes })
}

fn default_name(include: &[String]) -> String {
    match include {
        [only] => format!("{only}.zip"),
        _ => "archive.zip".to_owned(),
    }
}

fn free_name(dir: &Dir, wanted: &str, on_conflict: ZipConflict) -> Result<String> {
    if dir.meta(wanted.as_bytes()).is_err() {
        return Ok(wanted.to_owned());
    }
    if on_conflict == ZipConflict::Fail {
        return Err(Failure::conflict("already_exists", "there is already a file with this name"));
    }

    let stem = &wanted[..wanted.len() - 4];
    for number in 2..=MAX_RENAMES {
        let candidate = format!("{stem} ({number}).zip");
        if candidate.len() > path::MAX_SEGMENT_BYTES {
            break;
        }
        if dir.meta(candidate.as_bytes()).is_err() {
            return Ok(candidate);
        }
    }
    Err(Failure::conflict("already_exists", "every free name for this archive is taken"))
}

fn too_large() -> Failure {
    Failure::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "archive_too_large",
        "this would pack more into one archive than the panel allows",
    )
}

#[derive(Debug, Clone, Copy, Default)]
struct Tally {
    entries: u64,
    bytes: u64,
}

impl Tally {
    fn over_the_ceiling(&self) -> bool {
        self.entries > MAX_PACK_ENTRIES || self.bytes > MAX_PACK_BYTES
    }
}

fn tally(dir: &Dir, raw: &[u8], depth: usize, found: &mut Tally) {
    if found.over_the_ceiling() {
        return;
    }
    let Ok(meta) = dir.meta(raw) else { return };
    match meta.kind {
        Kind::File => {
            found.entries += 1;
            found.bytes = found.bytes.saturating_add(meta.size);
        }
        Kind::Directory if depth < MAX_DEPTH => {
            found.entries += 1;
            let Ok(sub) = dir.child(raw) else { return };
            let Ok(names) = sub.entries() else { return };
            for name in names {
                tally(&sub, &name, depth + 1, found);
            }
        }
        _ => {}
    }
}

pub struct Job {
    pub operations: Arc<Operations>,
    pub workspace: Workspace,
    pub operation: Id,
    pub plan: Plan,
}

pub fn start(job: Job) {
    tokio::spawn(async move {
        let operations = Arc::clone(&job.operations);
        let id = job.operation;
        match run(job).await {
            Ok(()) => {}
            Err(Halt::Cancelled) => {
                let _ = operations.cancelled(id).await;
            }
            Err(Halt::Ended) => {}
            Err(Halt::Failed(error)) => {
                tracing::warn!("archive {id} failed: {} — {}", error.code, error.message);
                let _ = operations.fail(id, error).await;
            }
        }
    });
}

async fn run(job: Job) -> std::result::Result<(), Halt> {
    let Job { operations, workspace, operation, plan } = job;
    wait_for_a_turn(&operations, operation).await?;

    for name in &plan.include {
        let inside = plan.parent.with_name(name);
        if let Err(err) = workspace.hand_back(&inside).await {
            tracing::warn!("{inside} was not handed back before it was packed: {err}");
        }
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let packed = {
        let root = clone_root(&workspace)?;
        let (plan, watched) = (plan.clone(), Arc::clone(&cancel));
        drive(&operations, operation, &cancel, move |progress| {
            pack(&root, &plan, &watched, &progress)
        })
        .await?
    };

    workspace.hand_back(&plan.archive()).await.map_err(|failure| not_handed_back(&failure))?;

    if packed.left_out > 0 {
        return Err(failed(
            "invalid_path",
            OperationErrorStep::Filesystem,
            format!(
                "{} entries were left out of the archive: their names are not UTF-8 or they \
                 nest too deep",
                packed.left_out
            ),
        ));
    }

    let _ = operations
        .advance(
            operation,
            Step {
                progress: Some(1.0),
                bytes_processed: Some(packed.bytes),
                files_processed: Some(packed.files),
                ..Step::default()
            },
        )
        .await;
    let _ = operations.finish(operation).await;
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Packed {
    files: u64,
    bytes: u64,
    left_out: u64,
}

fn pack(
    root: &Root,
    plan: &Plan,
    cancel: &AtomicBool,
    progress: &Sender<Step>,
) -> std::result::Result<Packed, Halt> {
    let dir = root.dir(&plan.parent).map_err(|err| trouble(&err, "the folder went away"))?;
    let home = dir.try_clone().map_err(|err| trouble(&err, "the folder went away"))?;
    let (part, file) =
        Part::create(home, &plan.target).map_err(|err| trouble(&err, "no part file"))?;

    let mut packer = Packer {
        zip: zip::ZipWriter::new(BufWriter::with_capacity(CHUNK, file)),
        cancel,
        progress,
        expected: plan.bytes,
        entries: 0,
        reported: 0,
        done: Packed::default(),
        buffer: vec![0; CHUNK],
    };
    for name in &plan.include {
        packer.add(&dir, name.as_bytes(), "", 0)?;
    }
    let done = packer.done;

    let buffered = packer.zip.finish().map_err(zip_fault)?;
    let file = buffered
        .into_inner()
        .map_err(|err| trouble(err.error(), "the archive could not be written"))?;
    file.sync_all().map_err(|err| trouble(&err, "the archive could not be synced"))?;
    drop(file);

    match part.commit(plan.target.as_bytes(), false) {
        Ok(()) => Ok(done),
        Err(err) if err.raw_os_error() == Some(libc::EEXIST) => Err(failed(
            "already_exists",
            OperationErrorStep::Filesystem,
            format!("{} appeared while the archive was being made", plan.archive()),
        )),
        Err(err) => Err(trouble(&err, "the archive stayed a part")),
    }
}

struct Packer<'a> {
    zip: zip::ZipWriter<BufWriter<File>>,
    cancel: &'a AtomicBool,
    progress: &'a Sender<Step>,
    expected: u64,
    entries: u64,
    reported: u64,
    done: Packed,
    buffer: Vec<u8>,
}

impl Packer<'_> {
    fn add(
        &mut self,
        dir: &Dir,
        raw: &[u8],
        inside: &str,
        depth: usize,
    ) -> std::result::Result<(), Halt> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(Halt::Cancelled);
        }
        let Ok(name) = std::str::from_utf8(raw) else {
            self.done.left_out += 1;
            return Ok(());
        };
        let meta = match dir.meta(raw) {
            Ok(meta) => meta,
            Err(err) if gone(&err) => return Ok(()),
            Err(err) => return Err(unreadable(&err, "an entry could not be looked at")),
        };
        let entry = format!("{inside}{name}");

        match meta.kind {
            Kind::Directory if depth >= MAX_DEPTH => self.done.left_out += 1,
            Kind::Directory => {
                let sub = match dir.child(raw) {
                    Ok(sub) => sub,
                    Err(err) if gone(&err) => return Ok(()),
                    Err(err) => return Err(unreadable(&err, "a directory could not be opened")),
                };
                let mut names =
                    sub.entries().map_err(|err| unreadable(&err, "a directory is unreadable"))?;
                names.sort_unstable();

                self.count()?;
                let options =
                    zip::write::SimpleFileOptions::default().last_modified_time(stamp(meta.modified));
                self.zip.add_directory(format!("{entry}/"), options).map_err(zip_fault)?;

                let below = format!("{entry}/");
                for child in names {
                    self.add(&sub, &child, &below, depth + 1)?;
                }
            }
            Kind::File => self.add_file(dir, raw, &entry)?,
            Kind::Symlink | Kind::Other => {}
        }
        Ok(())
    }

    fn add_file(&mut self, dir: &Dir, raw: &[u8], entry: &str) -> std::result::Result<(), Halt> {
        let file = match dir.open_read(raw) {
            Ok(file) => file,
            Err(err) if gone(&err) => return Ok(()),
            Err(err) => return Err(unreadable(&err, "a file could not be opened")),
        };
        let stat = file.metadata().map_err(|err| unreadable(&err, "a file could not be read"))?;
        if !stat.is_file() {
            return Ok(());
        }
        let size = stat.len();

        self.count()?;
        if self.done.bytes.saturating_add(size) > MAX_PACK_BYTES {
            return Err(over_the_ceiling());
        }

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(method_for(entry))
            .large_file(size >= u64::from(u32::MAX))
            .last_modified_time(stamp(stat.mtime()))
            .unix_permissions(stat.mode() & 0o777);
        self.zip.start_file(entry, options).map_err(zip_fault)?;

        let mut source = file.take(size);
        loop {
            if self.cancel.load(Ordering::Relaxed) {
                return Err(Halt::Cancelled);
            }
            let read = match source.read(&mut self.buffer) {
                Ok(read) => read,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(unreadable(&err, "a file could not be read")),
            };
            if read == 0 {
                break;
            }
            self.zip
                .write_all(&self.buffer[..read])
                .map_err(|err| trouble(&err, "the archive could not be written"))?;
            self.done.bytes += read as u64;
            if self.done.bytes - self.reported >= REPORT_EVERY {
                self.report(entry);
            }
        }

        self.done.files += 1;
        if self.done.files % 32 == 0 {
            self.report(entry);
        }
        Ok(())
    }

    fn count(&mut self) -> std::result::Result<(), Halt> {
        self.entries += 1;
        if self.entries > MAX_PACK_ENTRIES {
            return Err(over_the_ceiling());
        }
        Ok(())
    }

    fn report(&mut self, entry: &str) {
        self.reported = self.done.bytes;
        let _ = self.progress.blocking_send(Step {
            progress: Some(share(self.done.bytes, self.expected).min(0.99)),
            bytes_processed: Some(self.done.bytes),
            files_processed: Some(self.done.files),
            current_file: Some(format!("/{entry}")),
            ..Step::default()
        });
    }
}

fn method_for(entry: &str) -> zip::CompressionMethod {
    let lower = entry.to_ascii_lowercase();
    if ALREADY_PACKED.iter().any(|ending| lower.ends_with(ending)) {
        zip::CompressionMethod::Stored
    } else {
        zip::CompressionMethod::Deflated
    }
}

fn stamp(seconds: i64) -> zip::DateTime {
    time::OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(|moment| zip::DateTime::try_from(moment).ok())
        .unwrap_or_default()
}

fn gone(err: &std::io::Error) -> bool {
    matches!(err.raw_os_error(), Some(libc::ENOENT) | Some(libc::ENOTDIR))
}

fn unreadable(err: &std::io::Error, doing: &str) -> Halt {
    match err.raw_os_error() {
        Some(libc::EACCES) | Some(libc::EPERM) => failed(
            "file_not_accessible",
            OperationErrorStep::Filesystem,
            format!("{doing}: the game left it closed to the panel"),
        ),
        _ => trouble(err, doing),
    }
}

fn over_the_ceiling() -> Halt {
    failed(
        "archive_too_large",
        OperationErrorStep::Filesystem,
        "this would pack more into one archive than the panel allows",
    )
}

fn zip_fault(err: zip::result::ZipError) -> Halt {
    match err {
        zip::result::ZipError::Io(err) => trouble(&err, "the archive could not be written"),
        other => failed(
            "interrupted_while_applying",
            OperationErrorStep::Filesystem,
            format!("the archive could not be written: {other}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::testing::Sandbox;
    use tokio::sync::mpsc;

    fn root(sandbox: &Sandbox) -> Root {
        Root::open(sandbox.server_dir()).expect("the root opens")
    }

    fn asking(parent: &str, include: &[&str]) -> ZipRequest {
        ZipRequest {
            parent: parent.to_owned(),
            include: include.iter().map(|name| (*name).to_owned()).collect(),
            target: None,
            on_conflict: ZipConflict::Fail,
        }
    }

    fn names_in(bytes: Vec<u8>) -> Vec<String> {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("a readable zip");
        let mut names: Vec<String> =
            (0..zip.len()).map(|index| zip.by_index(index).unwrap().name().to_owned()).collect();
        names.sort();
        names
    }

    fn packed(sandbox: &Sandbox, plan: &Plan) -> Packed {
        let cancel = AtomicBool::new(false);
        let (sender, _receiver) = mpsc::channel(64);
        match pack(&root(sandbox), plan, &cancel, &sender) {
            Ok(done) => done,
            Err(halt) => panic!("the packing has to get through: {halt:?}"),
        }
    }

    #[test]
    fn a_folder_is_packed_beside_itself_with_its_own_name_on_top() {
        let sandbox = Sandbox::new();
        sandbox.write("world/level.dat", b"level");
        sandbox.write("world/region/r.0.0.mca", &[7u8; 5000]);
        sandbox.mkdir("world/empty");

        let plan = plan(&root(&sandbox), &asking("/", &["world"])).expect("a plan");
        assert_eq!(plan.target, "world.zip");
        assert_eq!(plan.archive().on_the_wire(), "/world.zip");
        assert_eq!(plan.bytes, 5005);

        let done = packed(&sandbox, &plan);
        assert_eq!(done.files, 2);
        assert_eq!(done.bytes, 5005);

        let bytes = std::fs::read(sandbox.server_dir().join("world.zip")).expect("the archive");
        assert_eq!(
            names_in(bytes),
            [
                "world/",
                "world/empty/",
                "world/level.dat",
                "world/region/",
                "world/region/r.0.0.mca"
            ]
        );
        let leftovers: Vec<_> = std::fs::read_dir(sandbox.server_dir())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.contains(".part."))
            .collect();
        assert!(leftovers.is_empty(), "no part file stays behind: {leftovers:?}");
    }

    #[test]
    fn a_selection_is_packed_relative_to_its_folder_and_the_contents_survive() {
        let sandbox = Sandbox::new();
        sandbox.write("plugins/a.yml", b"a: 1\n");
        sandbox.write("plugins/Essentials/config.yml", b"b: 2\n");
        sandbox.write("plugins/skip.txt", b"not asked for");

        let mut request = asking("/plugins", &["a.yml", "Essentials", "a.yml"]);
        request.target = Some("configs.zip".to_owned());
        let plan = plan(&root(&sandbox), &request).expect("a plan");
        assert_eq!(plan.include, ["a.yml", "Essentials"], "a name given twice is packed once");
        packed(&sandbox, &plan);

        let bytes = std::fs::read(sandbox.server_dir().join("plugins/configs.zip")).unwrap();
        assert_eq!(names_in(bytes.clone()), ["Essentials/", "Essentials/config.yml", "a.yml"]);

        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut text = String::new();
        zip.by_name("Essentials/config.yml").unwrap().read_to_string(&mut text).unwrap();
        assert_eq!(text, "b: 2\n");
    }

    #[test]
    fn a_link_is_not_followed_out_of_the_tree() {
        let sandbox = Sandbox::new();
        let outside = sandbox.data_dir().join("secret.txt");
        std::fs::write(&outside, b"panel secret").expect("a file outside the server");
        sandbox.write("logs/latest.log", b"line\n");
        std::os::unix::fs::symlink(&outside, sandbox.server_dir().join("logs/secret.txt"))
            .expect("a link out");
        std::os::unix::fs::symlink(sandbox.data_dir(), sandbox.server_dir().join("logs/up"))
            .expect("a directory link out");

        let plan = plan(&root(&sandbox), &asking("/", &["logs"])).expect("a plan");
        packed(&sandbox, &plan);

        let bytes = std::fs::read(sandbox.server_dir().join("logs.zip")).unwrap();
        assert_eq!(names_in(bytes), ["logs/", "logs/latest.log"]);

        let refused = super::plan(&root(&sandbox), &asking("/logs", &["secret.txt"])).unwrap_err();
        assert_eq!(refused.code(), "not_a_regular_file", "a link itself is not packed either");
    }

    #[test]
    fn a_taken_name_is_refused_or_numbered_as_asked() {
        let sandbox = Sandbox::new();
        sandbox.write("world/level.dat", b"x");
        sandbox.write("world.zip", b"an older one");
        sandbox.write("world (2).zip", b"and another");

        let refused = plan(&root(&sandbox), &asking("/", &["world"])).unwrap_err();
        assert_eq!(refused.code(), "already_exists");
        assert_eq!(refused.status(), StatusCode::CONFLICT);

        let mut request = asking("/", &["world"]);
        request.on_conflict = ZipConflict::Rename;
        assert_eq!(plan(&root(&sandbox), &request).unwrap().target, "world (3).zip");
    }

    #[test]
    fn what_cannot_be_packed_is_refused_before_a_run_starts() {
        let sandbox = Sandbox::new();
        sandbox.write("server.properties", b"x");
        sandbox.mkdir(WORK_DIR);
        let root = root(&sandbox);

        let cases = [
            (asking("/", &[]), "nothing_to_pack"),
            (asking("/", &["../etc"]), "invalid_name"),
            (asking("/", &["a/b"]), "invalid_name"),
            (asking("/", &["missing"]), "not_found"),
            (asking("/", &[WORK_DIR]), "not_found"),
            (asking("/server.properties", &["x"]), "not_a_directory"),
            (asking("/../..", &["x"]), "invalid_path"),
        ];
        for (request, code) in cases {
            let refused = plan(&root, &request).expect_err("no plan");
            assert_eq!(refused.code(), code, "{request:?}");
        }

        let mut named = asking("/", &["server.properties"]);
        named.target = Some("backup.tar".to_owned());
        assert_eq!(plan(&root, &named).unwrap_err().code(), "invalid_name");
        named.target = Some("sub/dir.zip".to_owned());
        assert_eq!(plan(&root, &named).unwrap_err().code(), "invalid_name");
        named.target = Some(".zip".to_owned());
        assert_eq!(plan(&root, &named).unwrap_err().code(), "invalid_name");
    }

    #[test]
    fn the_ceilings_are_counted_before_and_during_a_run() {
        assert!(!Tally { entries: MAX_PACK_ENTRIES, bytes: MAX_PACK_BYTES }.over_the_ceiling());
        assert!(Tally { entries: MAX_PACK_ENTRIES + 1, bytes: 0 }.over_the_ceiling());
        assert!(Tally { entries: 1, bytes: MAX_PACK_BYTES + 1 }.over_the_ceiling());
    }

    #[test]
    fn what_is_compressed_already_is_stored_and_the_rest_is_deflated() {
        assert_eq!(method_for("mods/sodium.jar"), zip::CompressionMethod::Stored);
        assert_eq!(method_for("logs/2024-01-01-1.log.gz"), zip::CompressionMethod::Stored);
        assert_eq!(method_for("world/region/r.0.0.mca"), zip::CompressionMethod::Deflated);
        assert_eq!(method_for("server.properties"), zip::CompressionMethod::Deflated);
    }

    #[test]
    fn a_cancelled_run_leaves_no_part_file_behind() {
        let sandbox = Sandbox::new();
        sandbox.write("world/level.dat", b"x");
        let plan = plan(&root(&sandbox), &asking("/", &["world"])).expect("a plan");

        let cancel = AtomicBool::new(true);
        let (sender, _receiver) = mpsc::channel(64);
        match pack(&root(&sandbox), &plan, &cancel, &sender) {
            Err(Halt::Cancelled) => {}
            _ => panic!("a cancel has to be seen"),
        }
        let names: Vec<_> = std::fs::read_dir(sandbox.server_dir())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["world"]);
    }
}
