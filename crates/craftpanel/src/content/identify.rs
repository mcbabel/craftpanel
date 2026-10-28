use std::collections::BTreeSet;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha512};
use sqlx::SqlitePool;

use super::modrinth::{Modrinth, Upstream};
use super::paths;
use super::store::{self, ItemRow};
use crate::model::{ContentSourceKind, Id};

pub fn unknown(row: &ItemRow) -> bool {
    row.project_id.is_none()
        && row.sha512.is_none()
        && row.source_kind != ContentSourceKind::ModrinthModpack
}

pub fn forget_link(row: &mut ItemRow) {
    row.sha512 = None;
    row.project_id = None;
    row.version_id = None;
    row.has_update = false;
    row.update_version_id = None;
    row.source_kind = ContentSourceKind::Local;
}

pub fn sha512_of(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a regular file"));
    }
    let mut hasher = Sha512::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub enum Failed {
    Database(sqlx::Error),
    Upstream(Upstream),
}

impl From<sqlx::Error> for Failed {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

pub async fn identify(
    pool: &SqlitePool,
    modrinth: &Modrinth,
    server: Id,
    root: &Path,
) -> Result<BTreeSet<String>, Failed> {
    let rows: Vec<ItemRow> = store::list(pool, server).await?.into_iter().filter(unknown).collect();
    if rows.is_empty() {
        return Ok(BTreeSet::new());
    }

    let places: Vec<(ItemRow, PathBuf)> = rows
        .into_iter()
        .filter_map(|row| {
            let full = paths::resolve_leaf(root, &row.file_path).ok()?;
            Some((row, full))
        })
        .collect();
    let hashed: Vec<(ItemRow, String)> = tokio::task::spawn_blocking(move || {
        places
            .into_iter()
            .filter_map(|(row, full)| sha512_of(&full).ok().map(|hash| (row, hash)))
            .collect()
    })
    .await
    .unwrap_or_default();
    if hashed.is_empty() {
        return Ok(BTreeSet::new());
    }

    let mut hashes: Vec<String> = hashed.iter().map(|(_, hash)| hash.clone()).collect();
    hashes.sort();
    hashes.dedup();
    let found = modrinth.versions_by_hash(&hashes).await.map_err(Failed::Upstream)?;

    let mut linked = BTreeSet::new();
    for (mut row, hash) in hashed {
        if let Some(version) = found.get(&hash) {
            row.project_id = Some(version.project_id.clone());
            row.version_id = Some(version.id.clone());
            let _ = modrinth.remember_version(version).await;
            linked.insert(version.project_id.clone());
        }
        row.sha512 = Some(hash);
        store::upsert(pool, &row).await?;
    }
    Ok(linked)
}
