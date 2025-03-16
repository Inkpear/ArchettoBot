use std::time::Duration;

use crate::cq_models::{CqMessage, MsgTarget};
use crate::crawler_models::{BiliInfo, BiliParams, Competition, CompetitionType};
use chrono::Utc;
use reqwest::{Client, Error as ReqwestError, Response};
use serde_json::Value;
use thiserror::Error;
use tokio;

pub struct HttpServices {
    bot_server_address: (String, u16),
    crawler_server_address: (String, u16),
    client: Client,
}

#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("Missing required parameter: {0}")]
    MissingParameter(&'static str),

    #[error("HTTP client creation failed: {0}")]
    ClientCreation(#[from] ReqwestError),
}

#[derive(Debug, Error)]
pub enum DataGetError {
    #[error("From api: {0} get data Falid: {1}")]
    GetDataError(&'static str, String),

    #[error("HTTP client creation failed: {0}")]
    ClientCreation(#[from] ReqwestError),
}

pub struct HttpServicesBuilder {
    bot_server_address: Option<(String, u16)>,
    crawler_server_address: Option<(String, u16)>,
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

    pub fn bot_server(mut self, addr: (&str, u16)) -> Self {
        self.bot_server_address = Some((addr.0.to_string(), addr.1));
        self
    }

    pub fn crawler_server(mut self, addr: (&str, u16)) -> Self {
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
    pub fn new(bot_server_address: (String, u16), crawler_server_address: (String, u16)) -> Self {
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

    pub async fn get_bilibili_info(
        &self,
        bili_data: &BiliParams,
    ) -> Result<BiliInfo, DataGetError> {
        let (ip, port) = &self.crawler_server_address;
        let response = self
            .client
            .post(format!("http://{}:{}/get_bilibili_info", ip, port))
            .json(bili_data)
            .send()
            .await?;

        if response.status().eq(&200) {
            Ok(response.json::<BiliInfo>().await?)
        } else {
            Err(DataGetError::GetDataError(
                "/get_competition_info",
                response.json::<Value>().await?["message"].to_string(),
            ))
        }
    }

    pub async fn get_competition_info(
        &self,
        cpt_type: &CompetitionType,
    ) -> Result<Vec<Competition>, Box<dyn std::error::Error + Send + Sync>> {
        let (ip, port) = &self.crawler_server_address;
        let type_ = match cpt_type {
            CompetitionType::All => "all",
            CompetitionType::Nowcoder => "nowcoder",
            CompetitionType::Codeforces => "codeforces",
            CompetitionType::AtCoder => "atcoder",
            CompetitionType::Leetcode => "leetcode",
            CompetitionType::Luogu => "luogu",
            CompetitionType::Lanqiao => "lanqiao",
        };

        let response = self
            .client
            .get(format!(
                "http://{}:{}/get_competition_info/{}",
                ip, port, type_
            ))
            .send()
            .await?;

        if response.status().eq(&200) {
            Ok(response
                .json::<Vec<Competition>>()
                .await?
                .into_iter()
                .filter(|competition| competition.start_time > Utc::now().timestamp())
                .collect())
        } else {
            Err(Box::new(DataGetError::GetDataError(
                "/get_competition_info",
                response.json::<Value>().await?["message"].to_string(),
            )))
        }
    }
}

#[tokio::test]
async fn test_send_private_message() {
    let service = HttpServices::builder()
        .bot_server(("localhost", 3000))
        .crawler_server(("localhost", 8086))
        .build()
        .unwrap();
    let message = MsgTarget::Private {
        user_id: 2754919327,
    }
    .new_message()
    .image("https://static.nowcoder.com/acm/images-acm/logo.png")
    .text("hello world");
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
    let message = MsgTarget::new_group_message(861318999).text("hello");

    let res = service.send_message(message).await;
    assert!(res.is_ok(), "{:#?}", res.err());

    let resp = res.unwrap();
    assert!(resp.status() == 200, "{:#?}", resp);
}

#[tokio::test]
async fn test_get_bili_info() {
    let service = HttpServices::builder()
        .bot_server(("localhost", 3000))
        .crawler_server(("localhost", 8086))
        .build()
        .unwrap();
    let bili_params = BiliParams::new("BV1PDKWeUEmX").only_info(true);

    let res = service.get_bilibili_info(&bili_params).await;
    assert!(res.is_ok(), "{:#?}", res.err());
}

#[tokio::test]
async fn test_get_competition_info() {
    let service = HttpServices::builder()
        .bot_server(("localhost", 3000))
        .crawler_server(("localhost", 8086))
        .build()
        .unwrap();

    let res = service.get_competition_info(&CompetitionType::Leetcode).await;
    assert!(res.is_ok(), "{:#?}", res.err());
    println!("{:#?}", res.unwrap());
}
