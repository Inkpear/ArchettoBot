use serde::{Deserialize, Deserializer};

use crate::Segment;

fn deserialize_segments<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Segment>, D::Error> {
    let list: Vec<serde_json::Value> = Deserialize::deserialize(d)?;
    Ok(list
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Segment>(v).ok())
        .collect())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Group,
    Private,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeType {
    GroupIncrease,
    GroupDecrease,
    GroupRecall,
    FriendRecall,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sender {
    pub user_id: i64,
    pub nickname: String,
    #[serde(default)]
    pub card: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageEvent {
    pub time: i64,
    pub self_id: i64,
    pub post_type: String,
    pub message_type: MessageType,
    pub user_id: i64,
    #[serde(default)]
    pub group_id: Option<i64>,
    pub message_id: i64,
    #[serde(deserialize_with = "deserialize_segments")]
    pub message: Vec<Segment>,
    pub raw_message: String,
    pub sender: Sender,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetaEvent {
    pub time: i64,
    pub self_id: i64,
    pub post_type: String,
    pub meta_event_type: String,
    #[serde(default)]
    pub sub_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NoticeEvent {
    pub time: i64,
    pub self_id: i64,
    pub post_type: String,
    pub notice_type: NoticeType,
    #[serde(default)]
    pub user_id: Option<i64>,
    #[serde(default)]
    pub group_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_segment_filtered() {
        let json = r#"{
            "time": 1,
            "self_id": 100,
            "post_type": "message",
            "message_type": "group",
            "user_id": 200,
            "group_id": 300,
            "message_id": 1,
            "message": [
                {"type": "text", "data": {"text": "hi"}},
                {"type": "miniapp", "data": {"data": "..."}},
                {"type": "at", "data": {"qq": "400"}}
            ],
            "raw_message": "hi [CQ:miniapp,data=...] [CQ:at,qq=400]",
            "sender": {"user_id": 200, "nickname": "tester"}
        }"#;
        let evt: MessageEvent = serde_json::from_str(json).unwrap();
        assert_eq!(evt.message.len(), 2); // miniapp skipped
        assert!(matches!(evt.message[0], Segment::Text { .. }));
        assert_eq!(evt.raw_message, "hi [CQ:miniapp,data=...] [CQ:at,qq=400]");
    }
}
