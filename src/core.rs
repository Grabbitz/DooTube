use crate::google_domains::{GOOGLE_DOMAINS, GOOGLE_EXACT_HOSTS};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Navigation {
    Search(String),
    Page(Url),
    Video(Url),
}

pub fn parse_input(input: &str) -> Result<Navigation, &'static str> {
    let value = input.trim();
    if value.is_empty() {
        return Err("Enter a search or YouTube link.");
    }
    let likely_url = value.contains("://")
        || value.starts_with("youtu.be/")
        || value.starts_with("youtube.com/")
        || value.starts_with("www.youtube.com/")
        || value.starts_with("m.youtube.com/")
        || value.starts_with("music.youtube.com/");
    if !likely_url {
        let scheme = value
            .split_once(':')
            .map(|(prefix, _)| prefix)
            .unwrap_or_default();
        if [
            "javascript",
            "file",
            "data",
            "mailto",
            "http",
            "https",
            "ftp",
        ]
        .iter()
        .any(|blocked| scheme.eq_ignore_ascii_case(blocked))
        {
            return Err("Only YouTube web links are supported.");
        }
        return Ok(Navigation::Search(value.to_owned()));
    }
    let candidate = if value.contains("://") {
        value.to_owned()
    } else {
        format!("https://{value}")
    };
    let mut url = Url::parse(&candidate).map_err(|_| "That link is not a valid YouTube URL.")?;
    if url.scheme() != "https" && url.scheme() != "http" {
        return Err("Only YouTube web links are supported.");
    }
    if url.scheme() == "http" {
        url.set_scheme("https")
            .map_err(|_| "Only secure YouTube web links are supported.")?;
    }
    if !url.username().is_empty() || url.password().is_some() || url.port().is_some() {
        return Err("YouTube links with credentials or custom ports are not supported.");
    }
    if !is_supported_youtube_url(&url) {
        return Err("Only YouTube links can be opened here.");
    }
    let host = url.host_str().unwrap_or_default();
    let host = host
        .strip_prefix("www.")
        .or_else(|| host.strip_prefix("m."))
        .unwrap_or(host);
    let path = url.path().trim_matches('/');
    let is_video = if host == "youtu.be" {
        !path.is_empty()
    } else if path == "watch" {
        url.query_pairs()
            .any(|(key, value)| key == "v" && !value.is_empty())
    } else {
        let mut segments = path.split('/');
        let (kind, id, rest) = (segments.next(), segments.next(), segments.next());
        matches!((kind,id,rest),(Some("shorts"|"embed"|"live"),Some(id),None) if !id.is_empty())
    };
    Ok(if is_video {
        Navigation::Video(url)
    } else {
        Navigation::Page(url)
    })
}
pub fn is_supported_youtube_url(url: &Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    let host = host
        .strip_prefix("www.")
        .or_else(|| host.strip_prefix("m."))
        .unwrap_or(host);
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            host,
            "youtube.com" | "music.youtube.com" | "youtu.be" | "youtube-nocookie.com"
        )
}

pub fn is_music_url(url: &Url) -> bool {
    is_supported_youtube_url(url) && url.host_str() == Some("music.youtube.com")
}

/// Google pages the official YouTube sign-in and sign-out flows pass through.
pub fn is_google_sign_in_url(url: &Url) -> bool {
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    if GOOGLE_EXACT_HOSTS.contains(&host) {
        return true;
    }
    if let Some(domain) = host.strip_prefix("accounts.") {
        return GOOGLE_DOMAINS.contains(&domain);
    }
    let domain = host.strip_prefix("www.").unwrap_or(host);
    url.path().starts_with("/accounts/") && GOOGLE_DOMAINS.contains(&domain)
}

/// Returns the YouTube video ID for watch, youtu.be, shorts, embed, and live links.
pub fn video_id(url: &Url) -> Option<String> {
    if !is_supported_youtube_url(url) {
        return None;
    }
    let host = url.host_str().unwrap_or_default();
    let host = host
        .strip_prefix("www.")
        .or_else(|| host.strip_prefix("m."))
        .unwrap_or(host);
    let mut segments = url.path().trim_matches('/').split('/');
    let id = match (host, segments.next(), segments.next(), segments.next()) {
        ("youtu.be", Some(id), None, None) => Some(id.to_owned()),
        (_, Some("watch"), None, None) => url
            .query_pairs()
            .find(|(key, _)| key == "v")
            .map(|(_, value)| value.into_owned()),
        (_, Some("shorts" | "embed" | "live"), Some(id), None) => Some(id.to_owned()),
        _ => None,
    };
    id.filter(|id| !id.is_empty())
}

/// Turns a YouTube or YouTube Music document title into a display title, or `None` for generic page titles.
pub fn clean_page_title(title: &str) -> Option<String> {
    let title = strip_notification_count(title.trim());
    let title = title
        .strip_suffix(" - YouTube Music")
        .or_else(|| title.strip_suffix(" - YouTube"))
        .unwrap_or(title)
        .trim();
    (!title.is_empty() && title != "YouTube" && title != "YouTube Music").then(|| title.to_owned())
}

/// Removes the unread-notification count YouTube prefixes to the document title for signed-in
/// accounts: `(1) `…`(99) ` or `(99+) `. Longer numbers, such as a year, belong to the title.
fn strip_notification_count(title: &str) -> &str {
    let Some((count, rest)) = title
        .strip_prefix('(')
        .and_then(|title| title.split_once(") "))
    else {
        return title;
    };
    let digits = count.strip_suffix('+').unwrap_or(count);
    if (1..=2).contains(&digits.len()) && digits.bytes().all(|byte| byte.is_ascii_digit()) {
        rest
    } else {
        title
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueItem {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl QueueItem {
    pub fn video_id(&self) -> Option<String> {
        Url::parse(&self.url).ok().as_ref().and_then(video_id)
    }
    /// Saved title, falling back to the video ID when the title was never captured.
    pub fn label(&self) -> String {
        self.title
            .clone()
            .or_else(|| self.video_id())
            .unwrap_or_else(|| "YouTube video".to_owned())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppState {
    pub up_next: Vec<QueueItem>,
}

impl AppState {
    /// Index of the saved item for the same video (or the same URL when no video ID exists).
    pub fn position(&self, url: &Url) -> Option<usize> {
        let id = video_id(url);
        self.up_next.iter().position(|item| match &id {
            Some(id) => item.video_id().as_ref() == Some(id),
            None => item.url == url.as_str(),
        })
    }
    pub fn add(&mut self, url: &Url, title: Option<String>) -> bool {
        if self.position(url).is_some() {
            return false;
        }
        self.up_next.push(QueueItem {
            url: url.as_str().to_owned(),
            title,
        });
        true
    }
    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.up_next.len() {
            return false;
        }
        self.up_next.remove(index);
        true
    }
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        if from >= self.up_next.len() || to >= self.up_next.len() {
            return false;
        }
        let item = self.up_next.remove(from);
        self.up_next.insert(to, item);
        true
    }
}

pub fn state_path() -> Result<PathBuf, &'static str> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support/DooTube/state.json"))
        .ok_or("Cannot locate your home folder; queue is not saved.")
}
pub fn load_state() -> Result<AppState, String> {
    let path = state_path().map_err(str::to_owned)?;
    load_state_at(&path)
}
fn load_state_at(path: &Path) -> Result<AppState, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "Saved Up Next data at {} is unreadable and was left untouched: {error}",
                path.display()
            )
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppState::default()),
        Err(error) => Err(format!("Saved Up Next data could not be read: {error}")),
    }
}
pub fn save_state(state: &AppState) -> std::io::Result<()> {
    let path = state_path().map_err(std::io::Error::other)?;
    save_state_at(state, &path)
}
fn save_state_at(state: &AppState, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(std::io::Error::other)?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_search_and_supported_youtube_pages_and_video_links() {
        assert_eq!(
            parse_input("cats and dogs").unwrap(),
            Navigation::Search("cats and dogs".into())
        );
        assert_eq!(
            parse_input("Rust: ownership").unwrap(),
            Navigation::Search("Rust: ownership".into())
        );
        assert_eq!(
            parse_input("NASA: live").unwrap(),
            Navigation::Search("NASA: live".into())
        );
        assert!(matches!(
            parse_input("https://www.youtube.com/watch?v=abc").unwrap(),
            Navigation::Video(_)
        ));
        assert!(matches!(
            parse_input("youtu.be/abc?t=3").unwrap(),
            Navigation::Video(_)
        ));
        assert!(matches!(
            parse_input("https://youtube.com/shorts/abc").unwrap(),
            Navigation::Video(_)
        ));
        assert!(matches!(
            parse_input("https://www.youtube-nocookie.com/embed/abc").unwrap(),
            Navigation::Video(_)
        ));
        assert!(matches!(
            parse_input("https://youtube.com/results?search_query=cats").unwrap(),
            Navigation::Page(_)
        ));
        assert!(matches!(
            parse_input("https://youtube.com/playlist?list=PL123").unwrap(),
            Navigation::Page(_)
        ));
        let Navigation::Video(url) =
            parse_input("https://youtube.com/watch?v=abc&list=PL123&t=45").unwrap()
        else {
            panic!("video URL rejected")
        };
        assert_eq!(
            url.as_str(),
            "https://youtube.com/watch?v=abc&list=PL123&t=45"
        );
    }
    #[test]
    fn rejects_non_youtube_and_unsafe_schemes() {
        for input in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://example.com/watch?v=x",
            "https://youtube.com.evil.test/watch?v=x",
            "https://user@youtube.com/watch?v=x",
            "https://youtube.com:8443/watch?v=x",
        ] {
            assert!(parse_input(input).is_err(), "accepted {input}");
        }
        assert!(
            matches!(parse_input("http://youtube.com/watch?v=abc").unwrap(), Navigation::Video(url) if url.scheme()=="https")
        );
    }
    #[test]
    fn music_links_are_youtube_links_and_sign_in_hosts_are_exact() {
        let Navigation::Video(music) =
            parse_input("music.youtube.com/watch?v=abc&list=RD1").unwrap()
        else {
            panic!("music video link rejected")
        };
        assert!(is_music_url(&music));
        assert_eq!(video_id(&music).as_deref(), Some("abc"));
        assert!(!is_music_url(
            &Url::parse("https://www.youtube.com/watch?v=abc").unwrap()
        ));
        for url in [
            "https://accounts.google.com/ServiceLogin?service=youtube",
            "https://accounts.google.co.th/accounts/SetSID",
            "https://accounts.google.com.au/accounts/CheckCookie",
            "https://accounts.google.co.uk/accounts/SetSID",
            "https://accounts.google.de/accounts/SetSID",
            "https://www.google.co.th/accounts/SetSID",
            "https://google.com.au/accounts/CheckCookie",
            "https://accounts.youtube.com/",
            "https://consent.google.com/",
            "https://consent.youtube.com/",
            "https://gds.google.com/",
        ] {
            assert!(
                is_google_sign_in_url(&Url::parse(url).unwrap()),
                "rejected {url}"
            );
        }
        for url in [
            "http://accounts.google.com/ServiceLogin",
            "https://accounts.google.com.evil.test/",
            "https://accounts.google.co.th.evil.test/accounts/SetSID",
            "https://accounts.google.evil.com/accounts/SetSID",
            "https://accounts-google.com/accounts/SetSID",
            "https://www.google.com/search?q=x",
            "https://google.com/search?q=x",
            "https://user@accounts.google.com/",
            "https://accounts.google.com:8443/accounts/SetSID",
        ] {
            assert!(
                !is_google_sign_in_url(&Url::parse(url).unwrap()),
                "allowed {url}"
            );
        }
        // Typed input stays limited to YouTube; sign-in pages are reached only through the flow.
        assert!(parse_input("https://accounts.google.com/ServiceLogin").is_err());
    }
    #[test]
    fn regional_google_sign_in_hosts_use_the_published_allowlist() {
        for domain in ["google.com", "google.co.th", "google.com.au", "google.de"] {
            assert!(GOOGLE_DOMAINS.contains(&domain), "missing {domain}");
        }
        assert!(!GOOGLE_DOMAINS.contains(&"google.co.de"));
        assert!(!GOOGLE_DOMAINS.contains(&"google.com.de"));

        for url in [
            "https://accounts.google.com/accounts/SetSID",
            "https://accounts.google.co.th/accounts/SetSID",
            "https://accounts.google.com.au/accounts/SetSID",
            "https://accounts.google.de/accounts/SetSID",
            "https://www.google.co.th/accounts/SetSID",
        ] {
            assert!(
                is_google_sign_in_url(&Url::parse(url).unwrap()),
                "rejected {url}"
            );
        }
        for url in [
            "https://accounts.google.co.de/accounts/SetSID",
            "https://accounts.google.com.de/accounts/SetSID",
            "https://accounts.google.abc.de/accounts/SetSID",
            "https://accounts.google.xyz.io/accounts/SetSID",
            "https://accounts.google.org/accounts/SetSID",
            "https://www.google.foo/accounts/SetSID",
            "https://www.google.co.th/search",
            "https://accounts.google.co.th.evil.test/accounts/SetSID",
            "http://accounts.google.com/accounts/SetSID",
            "https://user@accounts.google.com/accounts/SetSID",
            "https://accounts.google.com:8443/accounts/SetSID",
        ] {
            assert!(
                !is_google_sign_in_url(&Url::parse(url).unwrap()),
                "allowed {url}"
            );
        }
    }
    #[test]
    fn queue_add_remove_and_reorder_are_bounded() {
        let a = Url::parse("https://youtube.com/watch?v=a").unwrap();
        let b = Url::parse("https://youtube.com/watch?v=b").unwrap();
        let mut state = AppState::default();
        assert!(state.add(&a, None));
        assert!(!state.add(&a, None));
        assert!(state.add(&b, None));
        assert!(state.move_item(0, 1));
        assert_eq!(state.up_next[0].url, b.as_str());
        assert!(state.remove(1));
        assert!(!state.remove(4));
    }
    #[test]
    fn same_video_in_another_link_form_is_not_saved_twice() {
        let mut state = AppState::default();
        let watch = Url::parse("https://www.youtube.com/watch?v=abc&t=12").unwrap();
        assert!(state.add(&watch, Some("Clip".into())));
        for other in [
            "https://youtu.be/abc",
            "https://m.youtube.com/watch?list=PL1&v=abc",
        ] {
            assert_eq!(
                state.position(&Url::parse(other).unwrap()),
                Some(0),
                "{other}"
            );
        }
        let different = Url::parse("https://www.youtube.com/watch?v=abcd").unwrap();
        assert_eq!(state.position(&different), None);
        let page = Url::parse("https://www.youtube.com/watch").unwrap();
        assert_eq!(video_id(&page), None);
    }
    #[test]
    fn page_titles_drop_the_youtube_suffix_and_notification_count() {
        assert_eq!(
            clean_page_title("(2024) Me at the zoo - YouTube").as_deref(),
            Some("(2024) Me at the zoo")
        );
        assert_eq!(
            clean_page_title("Tutorial - YouTube Studio tips - YouTube").as_deref(),
            Some("Tutorial - YouTube Studio tips")
        );
        assert_eq!(
            clean_page_title("Song - YouTube Music").as_deref(),
            Some("Song")
        );
        assert_eq!(
            clean_page_title("(3) Me at the zoo - YouTube").as_deref(),
            Some("Me at the zoo")
        );
        assert_eq!(
            clean_page_title("(99+) (2024) Me at the zoo - YouTube").as_deref(),
            Some("(2024) Me at the zoo")
        );
        assert_eq!(clean_page_title("YouTube Music"), None);
        assert_eq!(clean_page_title("YouTube"), None);
        assert_eq!(clean_page_title("(1) YouTube"), None);
        assert_eq!(clean_page_title("  "), None);
    }
    #[test]
    fn saved_queue_round_trips_without_touching_user_home() {
        let path = test_state_path();
        let video = Url::parse("https://youtube.com/watch?v=abc&list=PL1&t=12").unwrap();
        let mut state = AppState::default();
        state.add(&video, Some("Clip".into()));
        save_state_at(&state, &path).unwrap();
        assert_eq!(load_state_at(&path).unwrap().up_next, state.up_next);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn malformed_saved_queue_is_reported_and_left_untouched() {
        let path = test_state_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let corrupted = b"{not valid json";
        fs::write(&path, corrupted).unwrap();
        assert!(load_state_at(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), corrupted);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    fn test_state_path() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "dootube-state-test-{}-{unique}",
                std::process::id()
            ))
            .join("state.json")
    }
}
