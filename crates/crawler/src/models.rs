use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Competition {
    pub link: String,
    pub name: String,
    pub start_time: i64, // unix timestamp
    pub duration: i32,   // seconds
    pub platform: String,
    #[serde(default)]
    pub notified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiliInfo {
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub description: String,
    pub video_url: Option<String>,
    pub duration: String,
    pub play_count: u64,
    pub like_count: u64,
    pub coin_count: u64,
    pub fav_count: u64,
    pub publish_date: String,
    pub cid: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn competition_serialize_roundtrip() {
        let c = Competition {
            link: "https://example.com/contest".into(),
            name: "Test Contest".into(),
            start_time: 1700000000,
            duration: 7200,
            platform: "Test".into(),
            notified: false,
        };
        let json = serde_json::to_string(&c).unwrap();
        let parsed: Competition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.link, c.link);
        assert_eq!(parsed.name, c.name);
        assert_eq!(parsed.duration, 7200);
        assert!(!parsed.notified);
    }

    #[test]
    fn competition_deserialize_defaults_notified() {
        let json = r#"{
            "link": "https://x.com",
            "name": "C",
            "start_time": 1,
            "duration": 1,
            "platform": "P"
        }"#;
        let c: Competition = serde_json::from_str(json).unwrap();
        assert!(!c.notified);
    }

    #[test]
    fn bili_info_serialize_roundtrip() {
        let info = BiliInfo {
            title: "Video Title".into(),
            author: "Author".into(),
            cover_url: "https://img.example.com/cover.jpg".into(),
            description: "A video about Rust".into(),
            video_url: Some("https://video.example.com/v.mp4".into()),
            duration: "10:30".into(),
            play_count: 10000,
            like_count: 500,
            coin_count: 200,
            fav_count: 300,
            publish_date: "2026-01-15".into(),
            cid: 123456789,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: BiliInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, info.title);
        assert_eq!(parsed.video_url, info.video_url);
        assert_eq!(parsed.play_count, 10000);
    }
}
