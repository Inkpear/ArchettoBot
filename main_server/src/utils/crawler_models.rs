use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct BiliParams {
    bv: String,
    only_info: bool,
    only_audio: bool,
}

impl BiliParams {
    pub fn new(bv: &str) -> Self {
        BiliParams {
            bv: bv.to_string(),
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
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum CompetitionType {
    All,
    Nowcoder,
    Codeforces,
    Atcoder,
    Leetcode,
    Luogu,
    Lanqiao
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct Competition {
    name: String,
    platform: CompetitionType,
    link: String,
    start_time: i64,
    duration: i64,
}


#[test]
fn test_deserialize_competition() {
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