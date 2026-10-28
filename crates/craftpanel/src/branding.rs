use sqlx::SqlitePool;

use crate::model::Timestamp;

pub const DEFAULT_NAME: &str = "CraftPanel";
pub const LONGEST_NAME: usize = 40;
pub const LARGEST_LOGO: usize = 512 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Brand {
    pub name: Option<String>,
    pub logo: Option<Logo>,
    pub updated_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logo {
    pub bytes: Vec<u8>,
    pub kind: LogoKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoKind {
    Png,
    Jpeg,
    Webp,
}

impl LogoKind {
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpeg)
        } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(Self::Webp)
        } else {
            None
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }

    fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            "image/webp" => Some(Self::Webp),
            _ => None,
        }
    }

    pub fn fits_mail(self) -> bool {
        matches!(self, Self::Png | Self::Jpeg)
    }
}

impl Brand {
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or(DEFAULT_NAME)
    }
}

pub async fn load(pool: &SqlitePool) -> sqlx::Result<Brand> {
    let row: Option<(Option<String>, Option<Vec<u8>>, Option<String>, Timestamp)> =
        sqlx::query_as("SELECT name, logo, logo_type, updated_at FROM branding WHERE id = 1")
            .fetch_optional(pool)
            .await?;
    let Some((name, logo, logo_type, updated_at)) = row else {
        return Ok(Brand::default());
    };
    let logo = match (logo, logo_type.as_deref().and_then(LogoKind::from_mime)) {
        (Some(bytes), Some(kind)) => Some(Logo { bytes, kind }),
        _ => None,
    };
    Ok(Brand { name, logo, updated_at: Some(updated_at) })
}

pub async fn set_name(pool: &SqlitePool, name: Option<&str>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO branding (id, name, updated_at) VALUES (1, ?, ?) \
         ON CONFLICT (id) DO UPDATE SET name = excluded.name, updated_at = excluded.updated_at",
    )
    .bind(name)
    .bind(Timestamp::now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_logo(pool: &SqlitePool, logo: Option<&Logo>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO branding (id, logo, logo_type, updated_at) VALUES (1, ?, ?, ?) \
         ON CONFLICT (id) DO UPDATE SET logo = excluded.logo, logo_type = excluded.logo_type, \
         updated_at = excluded.updated_at",
    )
    .bind(logo.map(|logo| logo.bytes.as_slice()))
    .bind(logo.map(|logo| logo.kind.mime()))
    .bind(Timestamp::now())
    .execute(pool)
    .await?;
    Ok(())
}

pub fn clean_name(raw: &str) -> Option<String> {
    let name = raw.trim();
    (!name.is_empty() && name.chars().count() <= LONGEST_NAME && !name.chars().any(char::is_control))
        .then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_png_jpeg_and_webp_pass_and_svg_never_does() {
        assert_eq!(LogoKind::sniff(b"\x89PNG\r\n\x1a\nrest"), Some(LogoKind::Png));
        assert_eq!(LogoKind::sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(LogoKind::Jpeg));
        assert_eq!(LogoKind::sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some(LogoKind::Webp));
        assert_eq!(LogoKind::sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script/></svg>"), None);
        assert_eq!(LogoKind::sniff(b"GIF89a"), None);
    }

    #[test]
    fn a_name_is_trimmed_and_kept_short() {
        assert_eq!(clean_name("  My Network  ").as_deref(), Some("My Network"));
        assert_eq!(clean_name("   "), None);
        assert_eq!(clean_name(&"x".repeat(41)), None);
        assert_eq!(clean_name("two\nlines"), None);
    }

    #[tokio::test]
    async fn nothing_set_means_craftpanel_and_both_parts_change_on_their_own() {
        let pool = crate::auth::harness::test_pool().await;
        assert_eq!(load(&pool).await.unwrap().name(), "CraftPanel");

        set_name(&pool, Some("Blockwerk")).await.unwrap();
        let logo = Logo { bytes: b"\x89PNG\r\n\x1a\nimage".to_vec(), kind: LogoKind::Png };
        set_logo(&pool, Some(&logo)).await.unwrap();
        let brand = load(&pool).await.unwrap();
        assert_eq!(brand.name(), "Blockwerk");
        assert_eq!(brand.logo, Some(logo));

        set_logo(&pool, None).await.unwrap();
        let brand = load(&pool).await.unwrap();
        assert_eq!((brand.name(), brand.logo.is_none()), ("Blockwerk", true));
        set_name(&pool, None).await.unwrap();
        assert_eq!(load(&pool).await.unwrap().name(), "CraftPanel");
    }
}
