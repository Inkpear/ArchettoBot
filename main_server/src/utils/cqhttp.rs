use reqwest::{Client, Error};
use serde_json::{Value, json};
use serde::{Serialize, Deserialize};

pub struct CqhttpServices {
    address: (String, u32),
    client: Client,
}

impl CqhttpServices {
    pub fn new(ip: &str, port: u32) -> Self {
        CqhttpServices {
            address: (ip.to_string(), port),
            client: Client::new()
        }
    }
}

#[derive(Serialize, Deserialize)]
enum MsgTarget {
    Private(String),
    Group(String)
}

#[derive(Serialize, Deserialize)]
struct Message {
    #[serde(rename="type")]
    type_: String,
    data: Value
}

impl Message {
    pub fn text(text: &str) -> Self {
        Message {
            type_: "text".into(),
            data: json!({
                "text": text
            })
        }
    }

    pub fn at(qq: &str) -> Self {
        Message {
            type_: "at".into(),
            data: json!({
                "qq": qq
            })
        }
    }

    pub fn image(image: &str) -> Self {
        Message {
            type_: "image".into(),
            data: json!({
                "file": image
            })
        }
    }

}

#[derive(Serialize, Deserialize)]
pub struct CqMessage {
    target: MsgTarget,
    #[serde(rename="message")]
    messages: Vec<Message>
}

impl CqMessage {
    pub fn new_group(group_id: &str) -> Self {
        CqMessage {
            target: MsgTarget::Group(group_id.into()),
            messages: vec![],
        }
    }

    pub fn at(mut self, qq: &str) -> Self {
        if let MsgTarget::Private(_) = self.target {
            panic!("不允许在私聊消息中@!")
        }
        self.messages.push(Message::at(qq));
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.messages.push(Message::text(text));
        self
    }

    pub fn image(mut self, image: &str) -> Self {
        self.messages.push(Message::image(image));
        self
    }
}