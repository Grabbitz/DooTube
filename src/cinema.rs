//! Data-only contract between the native host and its bundled Cinema Home document.
use crate::{
    core::{AppState, QueueItem, video_id, video_thumbnail},
    web::{Destination, Mode, Track},
};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeSeq};
use std::borrow::Cow;
use url::Url;

pub const MESSAGE_HANDLER: &str = "cinema";

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "lowercase", deny_unknown_fields)]
pub enum Action {
    Ready {},
    Open { url: String },
    Resume {},
    Queue {},
    Save {},
}

/// Identity of the originating WKWebView is checked by the host before this document check.
/// Neither subframes nor another local document may invoke the application bridge.
pub fn accepts_document(actual: &Url, expected: &Url, main_frame: bool) -> bool {
    main_frame && expected.scheme() == "file" && actual == expected
}

pub fn section(mode: Mode) -> &'static str {
    match mode {
        Mode::YouTube => "youtube",
        Mode::Music => "music",
    }
}

#[derive(Serialize)]
pub struct Current<'a> {
    pub url: &'a str,
    pub title: &'a str,
    pub thumbnail: Option<String>,
    pub saved: bool,
}

impl<'a> Current<'a> {
    pub fn new(url: &'a Url, title: &'a str, saved: bool) -> Option<Self> {
        video_id(url)?;
        Some(Self {
            url: url.as_str(),
            title,
            thumbnail: video_thumbnail(url),
            saved,
        })
    }
}

struct Entries<'a>(&'a [QueueItem]);

impl Serialize for Entries<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Entry<'a> {
            url: &'a str,
            resume_url: String,
            title: Cow<'a, str>,
            thumbnail: Option<String>,
            mode: &'static str,
            position: Option<u32>,
        }
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for item in self.0 {
            let mode = Url::parse(&item.url).map_or(Mode::YouTube, |url| Mode::for_url(&url));
            sequence.serialize_element(&Entry {
                url: &item.url,
                resume_url: item.resume_url(),
                title: item
                    .title
                    .as_deref()
                    .map_or_else(|| Cow::Owned(item.label()), Cow::Borrowed),
                thumbnail: item.thumbnail_url(),
                mode: section(mode),
                position: item.position,
            })?;
        }
        sequence.end()
    }
}

struct Destinations<'a>(&'a [Destination]);

impl Serialize for Destinations<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Link<'a> {
            title: &'a str,
            url: &'a str,
        }
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for destination in self.0 {
            sequence.serialize_element(&Link {
                title: destination.title,
                url: destination.url,
            })?;
        }
        sequence.end()
    }
}

/// Serialize directly into the JavaScript call buffer, without a second JSON string or queue copy.
pub fn state_script(
    mode: Mode,
    queue: &Result<AppState, String>,
    track: &Track,
    current: Option<Current<'_>>,
) -> String {
    #[derive(Serialize)]
    struct Playing<'a> {
        title: &'a str,
        artist: &'a str,
        artwork: Option<String>,
        playing: bool,
    }
    #[derive(Serialize)]
    struct Snapshot<'a> {
        mode: &'static str,
        queue: Entries<'a>,
        queue_error: Option<&'a str>,
        current: Option<Current<'a>>,
        track: Playing<'a>,
        destinations: Destinations<'a>,
    }
    let (items, error) = match queue {
        Ok(state) => (state.up_next.as_slice(), None),
        Err(error) => (&[][..], Some(error.as_str())),
    };
    let snapshot = Snapshot {
        mode: section(mode),
        queue: Entries(items),
        queue_error: error,
        current,
        track: Playing {
            title: &track.title,
            artist: &track.artist,
            artwork: track.artwork_url(),
            playing: track.playing,
        },
        destinations: Destinations(mode.destinations()),
    };
    let mut script = Vec::with_capacity(1024);
    script.extend_from_slice(b"window.dootube?.render(");
    serde_json::to_writer(&mut script, &snapshot).expect("Cinema state serializes as JSON");
    script.extend_from_slice(b");");
    String::from_utf8(script).expect("JSON and the call wrapper are UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_accepts_only_the_exact_bundled_main_document() {
        let expected =
            Url::parse("file:///Applications/DooTube.app/Contents/Resources/cinema/index.html")
                .unwrap();
        assert!(accepts_document(&expected, &expected, true));
        assert!(!accepts_document(&expected, &expected, false));
        for other in [
            "https://www.youtube.com/",
            "file:///Applications/DooTube.app/Contents/Resources/cinema/other.html",
            "file:///Applications/DooTube.app/Contents/Resources/cinema/index.html?untrusted=1",
            "file:///Applications/DooTube.app/Contents/Resources/cinema/index.html#untrusted",
            "file://other-host/Applications/DooTube.app/Contents/Resources/cinema/index.html",
            "about:blank",
        ] {
            assert!(
                !accepts_document(&Url::parse(other).unwrap(), &expected, true),
                "{other}"
            );
        }
    }

    #[test]
    fn bridge_does_not_accept_undeclared_commands_or_extra_arguments() {
        assert_eq!(
            serde_json::from_str::<Action>(r#"{"action":"save"}"#).unwrap(),
            Action::Save {}
        );
        for input in [
            r#"{"action":"eval","script":"1"}"#,
            r#"{"action":"open"}"#,
            r#"{"action":"open","url":"https://youtube.com/","script":"1"}"#,
            r#"{"action":"save","path":"/tmp/other"}"#,
        ] {
            assert!(serde_json::from_str::<Action>(input).is_err(), "{input}");
        }
    }

    #[test]
    fn snapshots_preserve_queue_order_resume_and_untrusted_titles_as_data() {
        let title = "\"});globalThis.injected=true;// <script>";
        let queue = Ok(AppState {
            up_next: vec![
                QueueItem {
                    url: "https://www.youtube.com/watch?v=abc&t=5".into(),
                    title: Some(title.into()),
                    position: Some(42),
                },
                QueueItem {
                    url: "https://music.youtube.com/watch?v=def".into(),
                    title: Some("Music track".into()),
                    position: None,
                },
            ],
        });
        let track = Track {
            artwork: "https://untrusted.invalid/cover.jpg".into(),
            ..Track::default()
        };
        let script = state_script(Mode::YouTube, &queue, &track, None);
        let json = script
            .strip_prefix("window.dootube?.render(")
            .unwrap()
            .strip_suffix(");")
            .unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(snapshot["queue"][0]["title"], title);
        assert_eq!(
            snapshot["queue"][0]["resume_url"],
            "https://www.youtube.com/watch?v=abc&t=42s"
        );
        assert_eq!(snapshot["queue"][1]["mode"], "music");
        assert!(snapshot["track"]["artwork"].is_null());
        assert!(snapshot["current"].is_null());
    }
}
