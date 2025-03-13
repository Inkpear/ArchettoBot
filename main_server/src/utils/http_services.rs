use std::time::Duration;

use cq_models::{CqMessage, MsgTarget};
use reqwest::{Client, Error as ReqwestError, Response};
use thiserror::Error;
use tokio;

mod cq_models;

pub struct HttpServices {
    bot_server_address: (String, u32),
    crawler_server_address: (String, u32),
    client: Client,
}

#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("Missing required parameter: {0}")]
    MissingParameter(&'static str),

    #[error("HTTP client creation failed: {0}")]
    ClientCreation(#[from] ReqwestError),
}

pub struct HttpServicesBuilder {
    bot_server_address: Option<(String, u32)>,
    crawler_server_address: Option<(String, u32)>,
    timeout: Duration,
}

impl HttpServicesBuilder {
    pub fn new() -> Self {
        Self {
            bot_server_address: None,
            crawler_server_address: None,
            timeout: Duration::from_secs(60),
        }
    }

    pub fn bot_server(mut self, addr: (&str, u32)) -> Self {
        self.bot_server_address = Some((addr.0.to_string(), addr.1));
        self
    }

    pub fn crawler_server(mut self, addr: (&str, u32)) -> Self {
        self.crawler_server_address = Some((addr.0.to_string(), addr.1));
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    pub fn build(self) -> Result<HttpServices, BuilderError> {
        let bot_addr = self
            .bot_server_address
            .ok_or(BuilderError::MissingParameter("bot_server_address"))?;

        let crawler_addr = self
            .crawler_server_address
            .ok_or(BuilderError::MissingParameter("crawler_server_address"))?;

        let client = Client::builder().timeout(self.timeout).build()?;

        Ok(HttpServices {
            bot_server_address: bot_addr,
            crawler_server_address: crawler_addr,
            client,
        })
    }
}

impl HttpServices {
    pub fn new(bot_server_address: (String, u32), crawler_server_address: (String, u32)) -> Self {
        HttpServices {
            bot_server_address,
            crawler_server_address,
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap(),
        }
    }

    pub fn builder() -> HttpServicesBuilder {
        HttpServicesBuilder::new()
    }

    pub async fn send_message(&self, message: CqMessage) -> Result<Response, ReqwestError> {
        let (ip, port) = &self.bot_server_address;
        let response = if let MsgTarget::Group { group_id: _ } = &message.target {
            self.client
                .post(format!("http://{}:{}/send_group_msg", ip, port))
                .json(&message)
                .send()
                .await?
        } else {
            self.client
                .post(format!("http://{}:{}/send_private_msg", ip, port))
                .json(&message)
                .send()
                .await?
        };

        Ok(response)
    }
}

#[tokio::test]
async fn test_send_private_message() {
    // use serde_json::json;

    let service = HttpServices::builder()
    .bot_server(("localhost", 3000))
    .crawler_server(("localhost", 8086))
    .build()
    .unwrap();
    let message = MsgTarget::Private {
        user_id: "2754919327".into(),
    }
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
    let service = HttpServices::builder()
    .bot_server(("localhost", 3000))
    .crawler_server(("localhost", 8086))
    .build()
    .unwrap();
    let message = MsgTarget::new_group("861318999").text("hello");

    let res = service.send_message(message).await;
    assert!(res.is_ok(), "{:#?}", res.err());

    let resp = res.unwrap();
    assert!(resp.status() == 200, "{:#?}", resp);
}
