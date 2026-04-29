use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Segment {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { file: String },
    #[serde(rename = "video")]
    Video { file: String },
    #[serde(rename = "at")]
    At { qq: String },
    #[serde(rename = "reply")]
    Reply { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message(pub Vec<Segment>);

impl Message {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn text(mut self, text: &str) -> Self {
        self.0.push(Segment::Text {
            text: text.to_owned(),
        });
        self
    }

    pub fn at(mut self, qq: &str) -> Self {
        self.0.push(Segment::At { qq: qq.to_owned() });
        self
    }

    pub fn image(mut self, file: &str) -> Self {
        self.0.push(Segment::Image {
            file: file.to_owned(),
        });
        self
    }

    pub fn video(mut self, file: &str) -> Self {
        self.0.push(Segment::Video {
            file: file.to_owned(),
        });
        self
    }

    pub fn reply(mut self, id: &str) -> Self {
        self.0.push(Segment::Reply { id: id.to_owned() });
        self
    }

    pub fn base64_image(self, b64: &str) -> Self {
        self.image(&format!("base64://{}", b64))
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in a merged forward message (合并转发).
#[derive(Debug, Clone, Serialize)]
pub struct ForwardNode {
    #[serde(rename = "type")]
    pub node_type: String,
    pub data: ForwardNodeData,
}

impl ForwardNode {
    pub fn new(user_id: i64, nickname: &str, content: Message) -> Self {
        Self {
            node_type: "node".to_owned(),
            data: ForwardNodeData {
                user_id: Some(user_id),
                uin: Some(user_id.to_string()),
                nickname: Some(nickname.to_owned()),
                name: None,
                content: serde_json::to_value(&content.0).unwrap_or_default(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ForwardNodeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub content: serde_json::Value,
}
