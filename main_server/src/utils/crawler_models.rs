use std::time::Duration;

use crate::models::TimeConverter;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Debug, PartialEq)]
pub struct BiliParams {
    bv: String,
    cookie: Option<String>,
    quality: bool,
    only_info: bool,
    only_audio: bool,
}

impl BiliParams {
    pub fn new(bv: &str) -> Self {
        BiliParams {
            bv: bv.to_string(),
            cookie: None,
            quality: false,
            only_audio: false,
            only_info: false,
        }
    }

    pub fn only_audio(mut self, only_audio: bool) -> Self {
        self.only_audio = only_audio;
        self
    }

    pub fn only_info(mut self, only_info: bool) -> Self {
        self.only_info = only_info;
        self
    }

    pub fn cookie(mut self, cookie: &str) -> Self {
        self.cookie = Some(cookie.to_string());
        self
    }

    pub fn quality(mut self, quality: bool) -> Self {
        self.quality = quality;
        self
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Eq)]
pub enum CompetitionType {
    All,
    Nowcoder,
    Codeforces,
    AtCoder,
    Leetcode,
    Luogu,
    Lanqiao,
    CQWUATP,
}

#[derive(Deserialize, PartialEq, Debug, Clone, Eq)]
pub struct Competition {
    pub start_time: i64,
    pub name: String,
    pub platform: CompetitionType,
    pub link: String,
    pub duration: u64,
}

impl Competition {
    pub fn fmt_string(&self) -> String {
        let time =
            TimeConverter::from_utc_to_utc8(&DateTime::from_timestamp(self.start_time, 0).unwrap());
        let duration = chrono::Duration::seconds(self.duration as i64);
        let hours = duration.num_hours();
        let remaining_seconds = self.duration - (hours as u64 * 3600);
        let minutes = remaining_seconds / 60;
        format!(
            "请注意! 以下比赛即将开始!\n\n{}\n\n比赛时间: {}至{}\n\n时长: {}小时{:02}分\n\n比赛链接: {}",
            self.name,
            time.format("%m-%d %H:%M"),
            (time + Duration::from_secs(self.duration)).format("%m-%d %H:%M"),
            hours,
            minutes,
            self.link
        )
    }

    pub fn face(&self) -> String {
        let current_dir = env::current_dir().unwrap();
        match self.platform {
            CompetitionType::All => "".to_string(),
            CompetitionType::Nowcoder => {
                let absolute_path = current_dir
                    .join("../data/logo/nowcoder_logo.png")
                    .canonicalize()
                    .unwrap();
                absolute_path.display().to_string()
            }
            CompetitionType::Codeforces => {
                "https://codeforces.org/s/29872/images/codeforces-sponsored-by-ton.png".to_string()
            }
            CompetitionType::AtCoder => {
                "https://img.atcoder.jp/logo/atcoder/logo_transparent.png".to_string()
            }
            CompetitionType::Leetcode => {
                let absolute_path = current_dir
                    .join("../data/logo/leetcode_logo.png")
                    .canonicalize()
                    .unwrap();
                absolute_path.display().to_string()
            }
            CompetitionType::Luogu => {
                "https://fecdn.luogu.com.cn/luogu/logo.png?0fdd294ff62e331d2f70e1a37ba4ee02"
                    .to_string()
            }
            CompetitionType::Lanqiao => {
                let absolute_path = current_dir
                    .join("../data/logo/lanqiao_logo.png")
                    .canonicalize()
                    .unwrap();
                absolute_path.display().to_string()
            }
            CompetitionType::CQWUATP => {
                "http://oj.cqwuc204.top/assets/img/acm.9347555f.jpg".to_string()
            }
        }
    }
}

impl PartialOrd for Competition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.start_time.cmp(&other.start_time))
    }
}

impl Ord for Competition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start_time.cmp(&other.start_time)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BiliInfo {
    pub bv: String,
    pub title: String,
    pub up: String,
    pub coin: String,
    pub date: String,
    pub fav: String,
    pub like: String,
    pub share: String,
    pub video_url: String,
    pub view: String,
}

#[test]
fn test_deserialize_competition() {
    // 测试比赛信息反序列化
    let competition = Competition {
        name: "第 152 场双周赛".into(),
        platform: CompetitionType::Leetcode,
        link: "https://leetcode.cn/contest/biweekly-contest-152".into(),
        start_time: 1742049000,
        duration: 5400,
    };
    let string = "{\"duration\": 5400,\"link\": \"https://leetcode.cn/contest/biweekly-contest-152\",\"name\": \"第 152 场双周赛\",\"platform\": \"Leetcode\",\"start_time\": 1742049000}";
    let cmp_target = serde_json::from_str::<Competition>(string).unwrap();

    assert_eq!(competition, cmp_target);
}

#[test]
fn test_serialize_bili_params() {
    // 测试bili_params参数序列化
    use serde_json::json;
    let bili_params = BiliParams::new("test");
    let cmp = json!({
        "bv": "test",
        "cookie": Option::<String>::None,
        "quality": false,
        "only_info": false,
        "only_audio": false,
    });

    assert_eq!(json!(bili_params), cmp);
}

#[test]
fn test_deserialize_bili_info() {
    // 测试BiliInfo反序列化
    let target = BiliInfo {
        bv: "BV1PDKWeUEmX".to_string(),
        coin: "1264".to_string(),
        date: "2025-02-13 21:10:23".to_string(),
        fav: "4991".to_string(),
        like: "1.9万".to_string(),
        share: "2073".to_string(),
        title: "⚡千早爱音看了自己都没绷住⚡".to_string(),
        up: "小默视频".to_string(),
        video_url: "https://www.bilibili.com/video/BV1PDKWeUEmX".to_string(),
        view: "14.8万".to_string(),
    };

    let cmp = serde_json::from_str::<BiliInfo>(
        r#"{
  "bv": "BV1PDKWeUEmX",
  "coin": "1264",
  "date": "2025-02-13 21:10:23",
  "fav": "4991",
  "like": "1.9万",
  "share": "2073",
  "title": "⚡千早爱音看了自己都没绷住⚡",
  "up": "小默视频",
  "video_url": "https://www.bilibili.com/video/BV1PDKWeUEmX",
  "view": "14.8万"
}"#,
    )
    .unwrap();

    assert_eq!(cmp, target);
}
