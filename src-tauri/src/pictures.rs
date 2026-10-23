//! VRChat pictures without logging in: public profile and avatar pages
//! include a picture (and name) for link previews (`og:image`, `og:title`).
//! We read those, shrink the image and cache it on disk.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use std::path::Path;
use std::time::{Duration, SystemTime};

const USER_AGENT: &str = concat!("PregnancyOSC/", env!("CARGO_PKG_VERSION"), " (desktop companion app)");
const REFRESH: Duration = Duration::from_secs(7 * 24 * 3600);
const MAX_IMAGE_BYTES: u64 = 12 << 20;
const SIZE: u32 = 128;

fn valid_id(id: &str, prefix: &str) -> bool {
    let Some(rest) = id.strip_prefix(prefix) else { return false };
    rest.len() == 36 && rest.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Only real VRChat user IDs, so nothing else ends up in the URL or a file name.
pub fn valid_user_id(id: &str) -> bool {
    valid_id(id, "usr_")
}

pub fn valid_avatar_id(id: &str) -> bool {
    valid_id(id, "avtr_")
}

/// The `content` of a `<meta ... name="key">` tag.
fn meta(html: &str, key: &str) -> Option<String> {
    let needle = format!("{key}\"");
    let mut from = 0;
    while let Some(i) = html[from..].find(&needle) {
        let at = from + i;
        let start = html[..at].rfind('<')?;
        let end = at + html[at..].find('>')?;
        let tag = &html[start..end];
        if let Some(c) = tag.find("content=\"") {
            let v = &tag[c + 9..];
            if let Some(q) = v.find('"') {
                return Some(unescape(&v[..q]));
            }
        }
        from = end;
    }
    None
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// Pulls the preview image URL out of the page HTML (VRChat servers only).
pub fn find_preview_image(html: &str) -> Option<String> {
    ["og:image", "twitter:image"]
        .iter()
        .filter_map(|k| meta(html, k))
        .find(|url| trusted(url) && !url.contains("default_social_image"))
}

/// Only download images from VRChat's own servers.
fn trusted(url: &str) -> bool {
    ["https://api.vrchat.cloud/", "https://files.vrchat.cloud/", "https://assets.vrchat.com/"]
        .iter()
        .any(|p| url.starts_with(p))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .user_agent(USER_AGENT)
        .build()
        .into()
}

fn fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < REFRESH)
}

fn to_data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

fn fetch_page(url: &str) -> Result<String, String> {
    agent().get(url).call().map_err(|e| e.to_string())?.body_mut().read_to_string().map_err(|e| e.to_string())
}

/// Downloads an image and shrinks it to a small square PNG.
fn thumbnail(url: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    agent()
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .as_reader()
        .take(MAX_IMAGE_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let thumb = img.resize_to_fill(SIZE, SIZE, image::imageops::FilterType::Triangle);
    let mut out = Cursor::new(Vec::new());
    thumb.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

/// Reads a cached picture, or makes it with `make`, falling back to an
/// older copy when offline.
fn cached(path: &Path, make: impl FnOnce() -> Result<Vec<u8>, String>) -> Result<Option<String>, String> {
    if fresh(path) {
        if let Ok(png) = std::fs::read(path) {
            return Ok(Some(to_data_url(&png)));
        }
    }
    match make() {
        Ok(png) => {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(path, &png);
            Ok(Some(to_data_url(&png)))
        }
        Err(e) => match std::fs::read(path) {
            Ok(png) => Ok(Some(to_data_url(&png))),
            Err(_) => Err(e),
        },
    }
}

/// A person's profile picture as a data URL.
pub fn get(dir: &Path, user_id: &str) -> Result<Option<String>, String> {
    if !valid_user_id(user_id) {
        return Ok(None);
    }
    cached(&dir.join(format!("{user_id}.png")), || {
        let html = fetch_page(&format!("https://vrchat.com/home/user/{user_id}"))?;
        thumbnail(&find_preview_image(&html).ok_or("no picture on their profile")?)
    })
}

/// What VRChat's public page says about an avatar.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarPreview {
    pub name: Option<String>,
    pub author: Option<String>,
    /// Thumbnail as a data URL.
    pub picture: Option<String>,
    /// False for private avatars and unknown IDs (VRChat shows a generic page).
    pub public: bool,
}

/// "Robot by VRChat" -> ("Robot", "VRChat").
pub fn split_title(title: &str) -> (Option<String>, Option<String>) {
    let t = title.trim();
    if t.is_empty() || t == "VRChat" {
        return (None, None);
    }
    match t.rsplit_once(" by ") {
        Some((name, author)) => (Some(name.trim().to_string()), Some(author.trim().to_string())),
        None => (Some(t.to_string()), None),
    }
}

/// An avatar's name and thumbnail from its public page.
pub fn avatar(dir: &Path, avatar_id: &str) -> Result<AvatarPreview, String> {
    if !valid_avatar_id(avatar_id) {
        return Err("That isn't an avatar ID. It looks like avtr_ followed by letters and numbers.".into());
    }
    let info_path = dir.join(format!("{avatar_id}.json"));
    let pic_path = dir.join(format!("{avatar_id}.png"));
    // Name and author are cached next to the picture.
    let mut info: AvatarPreview = std::fs::read_to_string(&info_path)
        .ok()
        .filter(|_| fresh(&info_path))
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    if !fresh(&info_path) {
        let html = fetch_page(&format!("https://vrchat.com/home/avatar/{avatar_id}"))?;
        let (name, author) = split_title(&meta(&html, "og:title").unwrap_or_default());
        let image = find_preview_image(&html);
        info = AvatarPreview { name, author, picture: None, public: image.is_some() };
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(&info_path, serde_json::to_string(&info).unwrap_or_default());
        if let Some(url) = image {
            if let Ok(png) = thumbnail(&url) {
                let _ = std::fs::write(&pic_path, &png);
            }
        }
    }
    info.picture = std::fs::read(&pic_path).ok().map(|png| to_data_url(&png));
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert!(valid_user_id("usr_c1644b5b-3ca4-45b4-97c6-a2a0de70d469"));
        assert!(!valid_user_id("usr_../../etc"));
        assert!(!valid_user_id("Ember Fox"));
        assert!(valid_avatar_id("avtr_c38a1615-5bf5-42b4-84eb-a8b6c37cbd11"));
        assert!(!valid_avatar_id("usr_c38a1615-5bf5-42b4-84eb-a8b6c37cbd11"));
    }

    #[test]
    fn finds_og_image() {
        let html = r#"<head><meta name="twitter:card" content="summary_large_image">
            <meta name="og:title" content="tupper">
            <meta name="og:image" content="https://api.vrchat.cloud/api/1/file/file_b7566f5c/1/"></head>"#;
        assert_eq!(find_preview_image(html).as_deref(), Some("https://api.vrchat.cloud/api/1/file/file_b7566f5c/1/"));
        assert_eq!(meta(html, "og:title").as_deref(), Some("tupper"));
    }

    #[test]
    fn ignores_other_hosts_and_the_generic_image() {
        let html = r#"<meta property="og:image" content="https://evil.example/x.png">"#;
        assert_eq!(find_preview_image(html), None);
        assert_eq!(find_preview_image("<html>nothing</html>"), None);
        let generic = r#"<meta name="og:image" content="https://assets.vrchat.com/www/images/default_social_image.jpg">"#;
        assert_eq!(find_preview_image(generic), None);
    }

    #[test]
    fn avatar_titles() {
        assert_eq!(split_title("Robot by VRChat"), (Some("Robot".into()), Some("VRChat".into())));
        assert_eq!(split_title("Fox by Bun by Bun"), (Some("Fox by Bun".into()), Some("Bun".into())));
        assert_eq!(split_title("VRChat"), (None, None));
        assert_eq!(unescape("A &quot;Robot&quot;"), "A \"Robot\"");
    }

    /// Talks to vrchat.com, so it only runs when asked: cargo test -- --ignored
    #[test]
    #[ignore]
    fn live_fetch() {
        let dir = std::env::temp_dir().join("pregnancy-osc-picture-test");
        let url = get(&dir, "usr_c1644b5b-3ca4-45b4-97c6-a2a0de70d469").unwrap().unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
        let a = avatar(&dir, "avtr_c38a1615-5bf5-42b4-84eb-a8b6c37cbd11").unwrap();
        assert_eq!(a.name.as_deref(), Some("Robot"));
        assert!(a.public && a.picture.is_some());
        let private = avatar(&dir, "avtr_1b9e2cfa-0a4e-4c5e-9d0a-5b1f1c1a0000").unwrap();
        assert!(!private.public && private.picture.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
