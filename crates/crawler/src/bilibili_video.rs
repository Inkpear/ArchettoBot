//! Bilibili video stream URL fetching.
//!
//! Fetches DASH video/audio stream URLs from the B站 player API,
//! which are then downloaded and merged by the bot.

use serde::Deserialize;

use crate::error::{CrawlerError, Result};

#[derive(Deserialize)]
struct PlayerApiResponse {
    code: i32,
    data: Option<PlayerData>,
}

#[derive(Deserialize)]
struct PlayerData {
    dash: Option<DashData>,
}

#[derive(Deserialize)]
struct DashData {
    video: Vec<StreamInfo>,
    audio: Vec<StreamInfo>,
}

#[derive(Deserialize)]
pub struct StreamInfo {
    pub id: u32,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "base_url")]
    pub base_url_alt: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

/// URLs needed to download a B站 video.
pub struct BiliVideoUrls {
    pub video_url: String,
    pub audio_url: String,
    pub quality: String,
}

/// Fetch video/audio stream URLs for a given BV number.
pub async fn get_video_urls(bv: &str, cid: i64, prefer_quality: bool) -> Result<BiliVideoUrls> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .referer(true)
        .build()?;

    let url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&fnval=4048&fourk=1",
        bv, cid
    );
    let resp: PlayerApiResponse = client
        .get(&url)
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await?
        .json()
        .await?;

    parse_player_response(&resp, prefer_quality)
}

fn parse_player_response(resp: &PlayerApiResponse, prefer_quality: bool) -> Result<BiliVideoUrls> {
    if resp.code != 0 {
        return Err(CrawlerError::Parse(format!(
            "Failed to get video URLs: code={}",
            resp.code
        )));
    }

    let dash = resp
        .data
        .as_ref()
        .and_then(|d| d.dash.as_ref())
        .ok_or_else(|| CrawlerError::Parse("No DASH data in player response".into()))?;

    let video = if prefer_quality {
        // Use highest resolution (first in list)
        dash.video.first()
    } else {
        // Use lowest resolution (last in list)
        dash.video.last()
    }
    .ok_or_else(|| CrawlerError::Parse("No video streams available".into()))?;

    let audio = dash
        .audio
        .first()
        .ok_or_else(|| CrawlerError::Parse("No audio streams available".into()))?;

    let quality = format!("{}x{}", video.width, video.height);

    Ok(BiliVideoUrls {
        video_url: if !video.base_url.is_empty() {
            video.base_url.clone()
        } else {
            video.base_url_alt.clone()
        },
        audio_url: if !audio.base_url.is_empty() {
            audio.base_url.clone()
        } else {
            audio.base_url_alt.clone()
        },
        quality,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_player_response_ok() {
        let json = serde_json::json!({
            "code": 0,
            "data": {
                "dash": {
                    "video": [
                        {"id": 80, "baseUrl": "https://example.com/video_1080p.m4s", "base_url": "", "width": 1920, "height": 1080},
                        {"id": 64, "baseUrl": "https://example.com/video_720p.m4s", "base_url": "", "width": 1280, "height": 720}
                    ],
                    "audio": [
                        {"id": 30280, "baseUrl": "https://example.com/audio.m4s", "base_url": "", "width": 0, "height": 0}
                    ]
                }
            }
        });
        let resp: PlayerApiResponse = serde_json::from_value(json).unwrap();
        let urls = parse_player_response(&resp, true).unwrap();
        assert!(urls.video_url.contains("1080p"));
        assert!(urls.audio_url.contains("audio"));
        assert_eq!(urls.quality, "1920x1080");
    }

    #[test]
    fn parse_player_prefers_low_quality() {
        let json = serde_json::json!({
            "code": 0,
            "data": {
                "dash": {
                    "video": [
                        {"id": 80, "baseUrl": "https://example.com/video_1080p.m4s", "base_url": "", "width": 1920, "height": 1080},
                        {"id": 64, "baseUrl": "https://example.com/video_720p.m4s", "base_url": "", "width": 1280, "height": 720}
                    ],
                    "audio": [
                        {"id": 30280, "baseUrl": "https://example.com/audio.m4s", "base_url": "", "width": 0, "height": 0}
                    ]
                }
            }
        });
        let resp: PlayerApiResponse = serde_json::from_value(json).unwrap();
        let urls = parse_player_response(&resp, false).unwrap();
        assert!(urls.video_url.contains("720p"));
    }
}
