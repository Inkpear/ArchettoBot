use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum MsgTarget {
    #[serde(rename_all = "snake_case")]
    Group { group_id: u64 },
    #[serde(rename_all = "snake_case")]
    Private { user_id: u64 },
}

impl MsgTarget {
    pub fn new_group(group_id: u64) -> Self {
        Self::Group { group_id }
    }

    pub fn new_private(user_id: u64) -> Self {
        Self::Private { user_id }
    }

    pub fn new_message(self) -> CqMessage {
        CqMessage {
            target: self,
            messages: vec![],
        }
    }

    pub fn new_group_message(group_id: u64) -> CqMessage {
        CqMessage {
            target: Self::Group {
                group_id,
            },
            messages: vec![],
        }
    }

    pub fn new_private_message(user_id: u64) -> CqMessage {
        CqMessage {
            target: Self::Private {
                user_id,
            },
            messages: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    #[serde(rename = "type")]
    type_: String,
    data: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CqMessage {
    #[serde(flatten)]
    pub target: MsgTarget,
    #[serde(rename = "message")]
    messages: Vec<Message>,
}

impl CqMessage {
    pub fn at(mut self, qq: &str) -> Self {
        if let MsgTarget::Private { user_id: _ } = self.target {
            panic!("不可使用在私聊中添加@!")
        }

        self.messages.push(Message {
            type_: "at".into(),
            data: json!({
                "qq": qq
            }),
        });
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.messages.push(Message {
            type_: "text".into(),
            data: json!({
                "text": text
            }),
        });
        self
    }

    pub fn image(mut self, image: &str) -> Self {
        self.messages.push(Message {
            type_: "image".into(),
            data: json!({
                "file": image
            }),
        });
        self
    }

    pub fn video(mut self, video: &str) -> Self {
        self.messages.push(Message {
            type_: "video".into(),
            data: json!({
                "file": video
            }),
        });
        self
    }

    pub fn reply(mut self, id: &str) -> Self {
        self.messages.push(Message {
            type_: "reply".into(),
            data: json!({
                "id": id
            }),
        });
        self
    }
}

#[test]
fn test_cqmessage() {
    let message = MsgTarget::Group {
        group_id: 123456,
    }
    .new_message()
    .at("all")
    .text("测试信息");
    let cmp_target = json!({
        "group_id": "123456",
        "message": [
            {
                "type": "at",
                "data": {
                    "qq": "all"
                }
            },
            {
                "type": "text",
                "data": {
                    "text": "测试信息"
                }
            }
        ]
    });

    assert_eq!(json!(message), cmp_target)
}
