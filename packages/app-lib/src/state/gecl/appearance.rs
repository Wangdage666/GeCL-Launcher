use crate::state::State;
use anyhow::{Context, bail};
use image::{DynamicImage, ImageDecoder, ImageReader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::LazyLock;

const KEY: &str = "gecl_visual_appearance_v1";
const MAX_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
static MUTATION: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub accent_color: String,
    pub background_id: Option<String>,
    pub background_enabled: bool,
    pub blur: u8,
    pub overlay: u8,
    pub surface_opacity: u8,
    pub position: Position,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            accent_color: "#176fb0".into(),
            background_id: None,
            background_enabled: true,
            blur: 8,
            overlay: 18,
            surface_opacity: 88,
            position: Position::Center,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    #[default]
    Center,
    Top,
    Bottom,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Background {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub preferences: Preferences,
    pub background: Option<Background>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowAppearance {
    pub theme: crate::state::Theme,
    pub advanced_rendering: bool,
    pub native_decorations: bool,
}

fn validate(preferences: &Preferences) -> anyhow::Result<()> {
    let color = preferences.accent_color.as_bytes();
    if color.len() != 7
        || color[0] != b'#'
        || !color[1..].iter().all(u8::is_ascii_hexdigit)
    {
        bail!("主题色无效，请使用六位十六进制颜色");
    }
    if preferences.blur > 24
        || preferences.overlay > 60
        || !(80..=100).contains(&preferences.surface_opacity)
    {
        bail!("背景参数超出范围，请重新调整");
    }
    if let Some(id) = &preferences.background_id {
        validate_id(id)?;
    }
    Ok(())
}

fn validate_id(id: &str) -> anyhow::Result<()> {
    if id.len() != 64 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
        bail!("背景图片编号无效，请重新选择图片");
    }
    Ok(())
}

async fn directory(state: &State) -> anyhow::Result<PathBuf> {
    let root = state.directories.settings_dir.join("appearance");
    tokio::fs::create_dir_all(&root).await?;
    if tokio::fs::symlink_metadata(&root)
        .await?
        .file_type()
        .is_symlink()
    {
        bail!("背景目录不能使用链接");
    }
    Ok(root)
}

async fn preferences(state: &State) -> anyhow::Result<Preferences> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_metadata WHERE key = ?")
            .bind(KEY)
            .fetch_optional(&state.pool)
            .await?;
    let preferences = match raw {
        Some(raw) => serde_json::from_str(&raw).context("无法读取外观设置")?,
        None => Preferences::default(),
    };
    validate(&preferences)?;
    Ok(preferences)
}

async fn appearance(
    state: &State,
    preferences: Preferences,
) -> anyhow::Result<Appearance> {
    let background = if let Some(id) = &preferences.background_id {
        let path = directory(state).await?.join(format!("{id}.jpg"));
        match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) if metadata.file_type().is_file() => {
                Some(Background {
                    id: id.clone(),
                    path,
                })
            }
            _ => None,
        }
    } else {
        None
    };
    Ok(Appearance {
        preferences,
        background,
    })
}

pub async fn get() -> anyhow::Result<Appearance> {
    let state = State::get().await?;
    appearance(&state, preferences(&state).await?).await
}

pub async fn set(
    mut value: Preferences,
    window: Option<WindowAppearance>,
) -> anyhow::Result<Appearance> {
    validate(&value)?;
    value.accent_color.make_ascii_lowercase();
    let _guard = MUTATION.lock().await;
    let state = State::get().await?;
    let result = appearance(&state, value).await?;
    if result.preferences.background_id.is_some() && result.background.is_none()
    {
        bail!("背景图片已不存在，请重新选择图片或移除背景");
    }
    let _settings_guard = state.lock_synced_options().await;
    let mut transaction = state.pool.begin().await?;
    if let Some(window) = window {
        sqlx::query("UPDATE settings SET theme = ?, advanced_rendering = ?, native_decorations = ?")
            .bind(window.theme.as_str()).bind(window.advanced_rendering).bind(window.native_decorations)
            .execute(&mut *transaction).await?;
    }
    sqlx::query("INSERT INTO app_metadata(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(KEY).bind(serde_json::to_string(&result.preferences)?).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn import_background(source: PathBuf) -> anyhow::Result<Background> {
    let state = State::get().await?;
    let root = directory(&state).await?;
    tokio::task::spawn_blocking(move || {
        let input =
            std::fs::File::open(&source).context("无法打开图片，请重新选择")?;
        if !input.metadata()?.is_file()
            || input.metadata()?.len() > MAX_SOURCE_BYTES
        {
            bail!("请选择不超过 32 MB 的图片文件");
        }
        let mut bytes = Vec::new();
        input.take(MAX_SOURCE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_SOURCE_BYTES {
            bail!("请选择不超过 32 MB 的图片文件");
        }
        let bytes = normalize(&bytes)?;
        let id = format!("{:x}", Sha256::digest(&bytes));
        let path = root.join(format!("{id}.jpg"));
        let mut temporary = tempfile::NamedTempFile::new_in(&root)?;
        std::io::Write::write_all(&mut temporary, &bytes)?;
        temporary.as_file().sync_all()?;
        temporary.persist(&path).map_err(|error| error.error)?;
        Ok(Background { id, path })
    })
    .await?
}

pub async fn discard_background(id: String) -> anyhow::Result<()> {
    validate_id(&id)?;
    let _guard = MUTATION.lock().await;
    let state = State::get().await?;
    if preferences(&state).await?.background_id.as_ref() == Some(&id) {
        return Ok(());
    }
    let path = directory(&state).await?.join(format!("{id}.jpg"));
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn normalize(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut reader =
        ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    if !matches!(
        reader.format(),
        Some(
            image::ImageFormat::Jpeg
                | image::ImageFormat::Png
                | image::ImageFormat::WebP
        )
    ) {
        bail!("请选择 JPG、PNG 或 WebP 图片");
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder =
        reader.into_decoder().context("无法读取图片，请更换图片")?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)
        .context("无法解码图片，图片可能损坏或尺寸过大")?;
    image.apply_orientation(orientation);
    if image.width() > 2560 || image.height() > 1440 {
        image = image.resize(2560, 1440, image::imageops::FilterType::Lanczos3);
    }
    let mut pixels = image.to_rgba8();
    for pixel in pixels.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for (channel, base) in [245_u16, 247, 250].into_iter().enumerate() {
            pixel[channel] = ((u16::from(pixel[channel]) * alpha
                + base * (255 - alpha)
                + 127)
                / 255) as u8;
        }
        pixel[3] = 255;
    }
    let image = DynamicImage::ImageRgba8(pixels).to_rgb8();
    let mut result = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut result, 88)
        .encode_image(&image)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_reject_css_and_path_injection_and_unsafe_transparency() {
        let mut value = Preferences::default();
        for color in ["red", "#ffffff;display:none", "#12345g"] {
            value.accent_color = color.into();
            assert!(validate(&value).is_err());
        }
        value = Preferences::default();
        value.background_id = Some("../outside".into());
        assert!(validate(&value).is_err());
        value.background_id = None;
        value.surface_opacity = 79;
        assert!(validate(&value).is_err());
        assert!(validate(&Preferences::default()).is_ok());
    }

    #[test]
    fn older_preferences_receive_readable_defaults() {
        let value: Preferences =
            serde_json::from_str(r##"{"accentColor":"#123456"}"##).unwrap();
        assert_eq!(value.surface_opacity, 88);
        assert_eq!(value.position, Position::Center);
        assert!(value.background_id.is_none());
    }

    #[test]
    fn background_is_bounded_and_transparency_is_flattened() {
        let image = DynamicImage::new_rgba8(3000, 1600);
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let normalized = normalize(bytes.get_ref()).unwrap();
        let decoded = image::load_from_memory(&normalized).unwrap().to_rgb8();
        assert!(decoded.width() <= 2560 && decoded.height() <= 1440);
        assert!(
            (i64::from(decoded.width()) * 8 - i64::from(decoded.height()) * 15)
                .abs()
                <= 15
        );
        assert!(decoded.get_pixel(0, 0).0.into_iter().all(|c| c > 235));
        assert!(normalize(b"not an image").is_err());
    }
}
