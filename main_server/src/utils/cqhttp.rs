use std::time::Duration;

use reqwest::{Client, Error, Response};
use cq_models::{CqMessage, MsgTarget};
use tokio;

mod cq_models;

pub struct CqhttpServices {
    address: (String, u32),
    client: Client,
}

impl CqhttpServices {
    pub fn new(ip: &str, port: u32) -> Self {
        CqhttpServices {
            address: (ip.to_string(), port),
            client: Client::builder().timeout(Duration::from_secs(60)).build().unwrap(),
        }
    }

    pub async fn send_message(&self, message: CqMessage) -> Result<Response, Error>{
        let (ip, port) = &self.address;
        let response = if let MsgTarget::Group { group_id: _ } = &message.target {
            self.client.post(format!("http://{}:{}/send_group_msg", ip, port)).json(&message).send().await?
        } else {
            self.client.post(format!("http://{}:{}/send_private_msg", ip, port)).json(&message).send().await?
        };

        Ok(response)
    }
}

#[tokio::test]
async fn test_send_private_message() {
    // use serde_json::json;

    let service = CqhttpServices::new("localhost", 3000);
    let message = MsgTarget::Private { user_id: "2754919327".into() }
    .new_message()
    .text("hello world");
    // println!("{}", json!(message));
    let res = service.send_message(message).await;
    assert!(res.is_ok(), "{:#?}", res.err());

    let resp = res.unwrap();
    assert!(resp.status() == 200, "{:#?}", resp);
}

#[tokio::test]
async fn test_send_group_message() {
    let service = CqhttpServices::new("localhost", 3000);
    let message = MsgTarget::new_group("861318999")
    .text("hello");

    let res = service.send_message(message).await;
    assert!(res.is_ok(), "{:#?}", res.err());

    let resp = res.unwrap();
    assert!(resp.status() == 200, "{:#?}", resp);
}