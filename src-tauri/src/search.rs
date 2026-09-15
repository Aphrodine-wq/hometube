use crate::downloads::{javascript_runtime, run_bounded_command};
use crate::models::YoutubeSearchItem;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::process::{Command, Stdio};
use std::time::Duration;

const SEARCH_LIMIT: u32 = 20;
const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_QUERY_LENGTH: usize = 200;

pub fn search_youtube(
    yt_dlp: &str,
    js_runtime: &str,
    raw_query: &str,
    youtube_auth_args: &[String],
) -> Result<Vec<YoutubeSearchItem>> {
    let query = raw_query.trim();
    if query.is_empty() {
        bail!("Type something to search YouTube");
    }
    if query.len() > MAX_QUERY_LENGTH {
        bail!("Keep the search under {MAX_QUERY_LENGTH} characters");
    }
    let mut command = Command::new(yt_dlp);
    command
        .args([
            "--flat-playlist",
            "--dump-single-json",
            "--no-warnings",
            "--no-colors",
            "--playlist-end",
            &SEARCH_LIMIT.to_string(),
        ])
        .args(youtube_auth_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(runtime) = javascript_runtime(js_runtime) {
        command.args(["--js-runtimes", &runtime]);
    }
    command.arg(format!("ytsearch{SEARCH_LIMIT}:{query}"));
    let output = run_bounded_command(command, SEARCH_TIMEOUT)
        .with_context(|| format!("Could not search YouTube with {yt_dlp}"))?;
    let value: Value =
        serde_json::from_slice(&output).context("yt-dlp returned invalid search metadata")?;
    Ok(parse_search_results(&value))
}

fn parse_search_results(value: &Value) -> Vec<YoutubeSearchItem> {
    let empty = Vec::new();
    let entries = value.get("entries").and_then(Value::as_array).unwrap_or(&empty);
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();
    for entry in entries {
        let Some(video_id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(title) = entry.get("title").and_then(Value::as_str) else {
            continue;
        };
        if !seen.insert(video_id.to_owned()) {
            continue;
        }
        let url = entry
            .get("url")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={video_id}"));
        let channel = entry
            .get("channel")
            .or_else(|| entry.get("uploader"))
            .and_then(Value::as_str)
            .unwrap_or("Unknown channel")
            .to_owned();
        let duration_secs = entry
            .get("duration")
            .and_then(Value::as_f64)
            .filter(|duration| duration.is_finite() && *duration > 0.0)
            .map(|duration| duration.round() as u64);
        items.push(YoutubeSearchItem {
            video_id: video_id.to_owned(),
            url,
            title: title.to_owned(),
            channel,
            duration_secs,
            thumbnail_url: best_thumbnail(entry),
        });
    }
    items
}

fn best_thumbnail(entry: &Value) -> Option<String> {
    let thumbnails = entry.get("thumbnails").and_then(Value::as_array)?;
    thumbnails
        .iter()
        .fold(None::<(u64, String)>, |best, thumbnail| {
            let url = thumbnail.get("url").and_then(Value::as_str)?;
            let width = thumbnail.get("width").and_then(Value::as_u64).unwrap_or(0);
            match best {
                Some((best_width, _)) if width < best_width => best,
                _ => Some((width, url.to_owned())),
            }
        })
        .map(|(_, url)| url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_blank_and_oversized_queries() {
        assert!(search_youtube("yt-dlp", "node", "   ", &[]).is_err());
        assert!(search_youtube("yt-dlp", "node", &"a".repeat(MAX_QUERY_LENGTH + 1), &[]).is_err());
    }

    #[test]
    fn parses_entries_into_items_with_the_widest_thumbnail() {
        let value = json!({
            "entries": [
                {
                    "id": "abc123",
                    "title": "First video",
                    "channel": "Channel One",
                    "duration": 91.4,
                    "thumbnails": [
                        {"url": "https://i.ytimg.com/small.jpg", "width": 120},
                        {"url": "https://i.ytimg.com/big.jpg", "width": 1280}
                    ]
                },
                {
                    "id": "def456",
                    "title": "Second video",
                    "uploader": "Channel Two",
                    "url": "https://www.youtube.com/watch?v=def456"
                },
                {"id": "abc123", "title": "Duplicate entry"},
                {"title": "Entry without an id"},
                {"id": "ghi789"}
            ]
        });
        let items = parse_search_results(&value);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].video_id, "abc123");
        assert_eq!(items[0].url, "https://www.youtube.com/watch?v=abc123");
        assert_eq!(items[0].channel, "Channel One");
        assert_eq!(items[0].duration_secs, Some(91));
        assert_eq!(items[0].thumbnail_url.as_deref(), Some("https://i.ytimg.com/big.jpg"));
        assert_eq!(items[1].video_id, "def456");
        assert_eq!(items[1].channel, "Channel Two");
        assert_eq!(items[1].duration_secs, None);
    }

    #[test]
    fn tolerates_missing_entries_collection() {
        assert!(parse_search_results(&json!({})).is_empty());
        assert!(parse_search_results(&Value::Null).is_empty());
    }
}
