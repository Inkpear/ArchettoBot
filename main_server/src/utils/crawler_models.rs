use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum CompetitionType {
    All,
    Nowcoder,
    Codeforces,
    Atcoder,
    Leetcode,
    Luogu,
    Lanqiao,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct Competition {
    name: String,
    platform: CompetitionType,
    link: String,
    start_time: i64,
    duration: i64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BiliInfo {
    bv: String,
    title: String,
    up: String,
    coin: String,
    date: String,
    fav: String,
    like: String,
    share: String,
    video_url: String,
    view: String,
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
