use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use futures::StreamExt;
use image::imageops::FilterType;
use image::{DynamicImage, ImageError, ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};

use super::{announced, as_disk_trouble, gone, joined, refuse_a_form, workspace_for, OfServer};
use crate::audit::{self, Event};
use crate::auth::access::Access;
use crate::auth::disk::{self, Disks};
use crate::auth::error::{Failure, Result};
use crate::auth::Caller;
use crate::files::{self, Kind, Part, RelPath, Workspace};
use crate::model::{Id, Permission};
use crate::ops::Operations;
use crate::AppState;

const NAME: &str = "server-icon.png";
const SIDE: u32 = 64;

const MAX_UPLOAD_BYTES: usize = 4 * 1024 * 1024;
const MAX_STORED_BYTES: u64 = 1024 * 1024;
const MAX_SOURCE_SIDE: u32 = 4096;
const MAX_DECODE_BYTES: u64 = 128 * 1024 * 1024;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const REVALIDATE: &str = "private, no-cache";

pub(super) async fn read(
    State(state): State<AppState>,
    caller: Caller,
    OfServer(server): OfServer,
    headers: HeaderMap,
) -> Result<Response> {
    let (_, workspace) = workspace_for(&state, &caller, server, Permission::BaseRead).await?;
    let stored = tokio::task::spawn_blocking(move || stored(&workspace)).await.map_err(joined)?;
    let bytes = stored?.ok_or_else(no_icon)?;

    let tag = etag(&bytes);
    let asked = headers.get(IF_NONE_MATCH).and_then(|value| value.to_str().ok());
    if asked.is_some_and(|asked| asked.split(',').any(|one| one.trim() == tag)) {
        return Ok((StatusCode::NOT_MODIFIED, [(ETAG, tag), (CACHE_CONTROL, REVALIDATE.to_owned())])
            .into_response());
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "image/png")
        .header("x-content-type-options", "nosniff")
        .header(CONTENT_LENGTH, bytes.len())
        .header(ETAG, tag)
        .header(CACHE_CONTROL, REVALIDATE)
        .body(Body::from(bytes))
        .map_err(|err| Failure::internal(anyhow::Error::new(err)))
}

pub(super) async fn write(
    State(state): State<AppState>,
    caller: Caller,
    OfServer(server): OfServer,
    Extension(operations): Extension<Arc<Operations>>,
    Extension(disks): Extension<Disks>,
    request: Request,
) -> Result<StatusCode> {
    refuse_a_form(request.headers())?;
    let (seat, workspace) = settable(&state, &caller, server, &operations).await?;
    if announced(request.headers()).is_some_and(|length| length > MAX_UPLOAD_BYTES as u64) {
        return Err(too_large());
    }

    let mut upload = Vec::new();
    let mut body = request.into_body().into_data_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|err| {
            Failure::invalid_request(format!("the body stopped early: {err}"))
        })?;
        if upload.len() + chunk.len() > MAX_UPLOAD_BYTES {
            return Err(too_large());
        }
        upload.extend_from_slice(&chunk);
    }

    let png = tokio::task::spawn_blocking(move || to_icon(&upload)).await.map_err(joined)??;
    disk::guard(&state.pool, &disks, seat.owner_id, png.len() as u64).await?;

    let placed = tokio::task::spawn_blocking(move || {
        place(&workspace, &png).map(|replaced| (replaced, workspace))
    })
    .await
    .map_err(joined)?;
    let (replaced, workspace) = placed?;

    let at = icon_path();
    workspace.hand_back(&at).await?;
    let path = at.on_the_wire();
    let event = match replaced {
        true => Event::FileEdited { path },
        false => Event::FileUploaded { path },
    };
    audit::record(&state.pool, seat, &caller, event).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn remove(
    State(state): State<AppState>,
    caller: Caller,
    OfServer(server): OfServer,
    Extension(operations): Extension<Arc<Operations>>,
) -> Result<StatusCode> {
    let (seat, workspace) = settable(&state, &caller, server, &operations).await?;
    tokio::task::spawn_blocking(move || {
        let dir = workspace
            .root()
            .dir(&RelPath::root())
            .map_err(|err| files::fault(&err, "not_found"))?;
        match dir.meta(NAME.as_bytes()) {
            Ok(meta) if meta.kind == Kind::Directory => {
                Err(Failure::bad_request("not_a_regular_file", "the icon is a directory"))
            }
            Ok(_) => {
                dir.unlink(NAME.as_bytes()).map_err(|err| files::fault(&err, "icon_not_found"))
            }
            Err(err) if gone(&err) => Err(no_icon()),
            Err(err) => Err(files::fault(&err, "icon_not_found")),
        }
    })
    .await
    .map_err(joined)??;

    let path = icon_path().on_the_wire();
    audit::record(&state.pool, seat, &caller, Event::FileDeleted { path }).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn settable(
    state: &AppState,
    caller: &Caller,
    server: Id,
    operations: &Operations,
) -> Result<(Access, Workspace)> {
    let seat = workspace_for(state, caller, server, Permission::Advanced).await?;
    operations
        .guard_write(server)
        .await
        .map_err(|fault| Failure::new(fault.status(), fault.code(), fault.message().to_owned()))?;
    Ok(seat)
}

fn stored(workspace: &Workspace) -> Result<Option<Vec<u8>>> {
    let file = match workspace.root().open_read(&icon_path()) {
        Ok(file) => file,
        Err(err) if gone(&err) => return Ok(None),
        Err(err) => return Err(files::fault(&err, "icon_not_found")),
    };
    let meta = file.metadata().map_err(|err| files::fault(&err, "icon_not_found"))?;
    if !meta.is_file() || meta.len() > MAX_STORED_BYTES {
        return Ok(None);
    }

    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(MAX_STORED_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| files::fault(&err, "icon_not_found"))?;
    if bytes.len() as u64 > MAX_STORED_BYTES || !bytes.starts_with(PNG_SIGNATURE) {
        return Ok(None);
    }
    Ok(Some(bytes))
}

// Minecraft only shows a 64×64 PNG; anything else is cropped to a square from the middle.
fn to_icon(upload: &[u8]) -> Result<Vec<u8>> {
    let mut reader =
        ImageReader::new(Cursor::new(upload)).with_guessed_format().map_err(|_| unsupported())?;
    match reader.format() {
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::WebP) => {}
        _ => return Err(unsupported()),
    }

    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_SIDE);
    limits.max_image_height = Some(MAX_SOURCE_SIDE);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);

    let picture = reader.decode().map_err(|err| match err {
        ImageError::Limits(_) => too_large(),
        _ => Failure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_image",
            "this picture could not be read",
        ),
    })?;
    let icon = match (picture.width(), picture.height()) {
        (SIDE, SIDE) => picture,
        _ => picture.resize_to_fill(SIDE, SIDE, FilterType::Lanczos3),
    };

    let mut png = Vec::new();
    DynamicImage::ImageRgba8(icon.to_rgba8())
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|err| Failure::internal(anyhow::Error::new(err).context("encoding the icon")))?;
    Ok(png)
}

fn place(workspace: &Workspace, png: &[u8]) -> Result<bool> {
    let dir =
        workspace.root().dir(&RelPath::root()).map_err(|err| files::fault(&err, "not_found"))?;
    let replaced = dir.meta(NAME.as_bytes()).is_ok();

    let (part, mut file) = Part::create(dir, NAME).map_err(|err| files::fault(&err, "not_found"))?;
    file.write_all(png).and_then(|()| file.sync_all()).map_err(as_disk_trouble)?;
    drop(file);
    part.commit(NAME.as_bytes(), true).map_err(|err| files::fault(&err, "not_found"))?;
    Ok(replaced)
}

fn icon_path() -> RelPath {
    RelPath::root().child(NAME).expect("a plain file name")
}

fn etag(bytes: &[u8]) -> String {
    format!("\"{}\"", hex::encode(&Sha256::digest(bytes)[..16]))
}

fn no_icon() -> Failure {
    Failure::not_found("icon_not_found", "this server has no icon")
}

fn unsupported() -> Failure {
    Failure::new(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "unsupported_image",
        "the icon has to be a PNG, JPEG, GIF or WebP picture",
    )
}

fn too_large() -> Failure {
    Failure::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "icon_too_large",
        "this picture is larger than an icon may be",
    )
}
