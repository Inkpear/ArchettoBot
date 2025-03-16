use std::{env, time::Duration};

use actix_web::web;
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    crawler_models::{BiliParams, Competition},
    models::{FuncScope, TimeConverter},
    state::AppState,
};

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
            target: Self::Group { group_id },
            messages: vec![],
        }
    }

    pub fn new_private_message(user_id: u64) -> CqMessage {
        CqMessage {
            target: Self::Private { user_id },
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
                "file": video,
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

pub enum MessageType {
    Normal,
    GroupIncrease,
    GroupDecrease,
    GroupRecall,
    FriendRecall,
    Unknow,
}

pub struct MessageParser {
    pub raw_message: Value,
}

impl MessageParser {
    pub fn new(raw_message: Value) -> Self {
        Self { raw_message }
    }

    pub fn get_type(&self) -> MessageType {
        if let Some(_) = self.raw_message.get("message") {
            return MessageType::Normal;
        } else if let Some(value) = self.raw_message.get("notice_type") {
            let notice_type = serde_json::from_value::<String>(value.clone()).unwrap();
            return match notice_type.as_str() {
                "group_increase" => MessageType::GroupIncrease,
                "group_decrease" => MessageType::GroupDecrease,
                "group_recall" => MessageType::GroupRecall,
                "friend_recall" => MessageType::GroupRecall,
                _ => MessageType::Unknow,
            };
        }

        MessageType::Unknow
    }

    pub fn get_group_user_id(&self) -> Option<(u64, u64)> {
        if let (Some(group_value), Some(user_value)) = (
            self.raw_message.get("group_id"),
            self.raw_message.get("user_id"),
        ) {
            return Some((
                serde_json::from_value(group_value.clone()).unwrap(),
                serde_json::from_value(user_value.clone()).unwrap(),
            ));
        }

        None
    }

    pub fn get_user_id(&self) -> Option<u64> {
        if let Some(user_value) = self.raw_message.get("user_id") {
            return Some(serde_json::from_value(user_value.clone()).unwrap());
        }

        None
    }

    pub fn get_group_id(&self) -> Option<u64> {
        if let Some(group_value) = self.raw_message.get("group_id") {
            return Some(serde_json::from_value(group_value.clone()).unwrap());
        }

        None
    }

    pub fn messages(&self) -> Option<CqMessage> {
        if let None = self.raw_message.get("message") {
            return None;
        }

        let message = serde_json::from_value::<Vec<Message>>(
            self.raw_message.get("message").unwrap().clone(),
        )
        .unwrap();
        if let Some(group_id) = self.raw_message.get("group_id") {
            let group_id = serde_json::from_value::<u64>(group_id.clone()).unwrap();
            Some(CqMessage {
                target: MsgTarget::new_group(group_id),
                messages: message,
            })
        } else {
            let user_id =
                serde_json::from_value::<u64>(self.raw_message.get("user_id").unwrap().clone())
                    .unwrap();
            Some(CqMessage {
                target: MsgTarget::new_private(user_id),
                messages: message,
            })
        }
    }
}

pub struct MessageHandler;

impl MessageHandler {
    pub async fn handle(app_state: web::Data<AppState>, raw_msg: Value) {
        let parser = MessageParser::new(raw_msg);

        match parser.get_type() {
            MessageType::Normal => {
                info!("接收: {:?}", parser.messages().unwrap());
                Self::normal_message(app_state, parser).await;
            }
            MessageType::GroupIncrease => {
                let (group_id, user_id) = parser.get_group_user_id().unwrap();
                info!("接收: {} 加入群聊:{}", user_id, group_id);
            }
            MessageType::GroupDecrease => {
                let (group_id, user_id) = parser.get_group_user_id().unwrap();
                info!("接收: {} 退出群聊:{}", user_id, group_id);
            }
            MessageType::GroupRecall => {
                let (group_id, user_id) = parser.get_group_user_id().unwrap();
                info!("接收: [group({})][user({})]撤回一条消息", group_id, user_id);
            }
            MessageType::FriendRecall => {
                let user_id = parser.get_user_id().unwrap();
                info!("接收: [friend][user({})]撤回一条消息", user_id);
            }
            MessageType::Unknow => warn!("未知消息类型!\n{}", parser.raw_message),
        }
    }

    pub async fn normal_message(app_state: web::Data<AppState>, parser: MessageParser) {
        let messages = parser.messages().unwrap();
        let user_id = parser.get_user_id().unwrap();
        let target = parser
            .get_group_id()
            .and_then(|group_id| Some(MsgTarget::new_group(group_id)))
            .or_else(|| Some(MsgTarget::new_private(user_id)))
            .unwrap();

        if !app_state.func_scope_services.contains(&target) {
            app_state
                .func_scope_services
                .insert(target.clone(), FuncScope::new());
            let _ = app_state.func_scope_services.save();
        }

        let func_scope = app_state.func_scope_services.get_value(&target);

        for i in messages.messages {
            match i.type_.as_str() {
                "text" => {
                    let target = target.clone();
                    let text =
                        serde_json::from_value::<String>(i.data.get("text").unwrap().clone())
                            .unwrap();
                    let args = text
                        .split(" ")
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>();
                    if args.is_empty() {
                        continue;
                    }

                    if func_scope.bili_parse {
                        let regx = Regex::new(r"BV[a-zA-Z0-9]{10}")
                            .unwrap()
                            .captures(&args[0])
                            .and_then(|cap| cap.get(0));
                        if let Some(bv) = regx {
                            let bv = bv.as_str().to_string();
                            debug!("{}", bv);
                            Self::handle_bili_info(app_state.clone(), &bv, target.clone()).await;
                            return;
                        }
                    }

                    if func_scope.competition && args[0].eq("查询比赛") {
                        Self::handle_competition_info(
                            app_state.clone(),
                            target.clone(),
                            args.clone(),
                        )
                        .await;
                        return;
                    }
                }
                "json" => {
                    let value = serde_json::from_str::<Value>(
                        &serde_json::from_value::<String>(i.data.get("data").unwrap().clone())
                            .unwrap(),
                    )
                    .unwrap();
                    let title = value["meta"]["detail_1"]["title"].as_str().unwrap();
                    if title.eq("哔哩哔哩") {
                        let bv = value["meta"]["detail_1"]["qqdocurl"].as_str().unwrap();
                        Self::handle_bili_info(app_state.clone(), bv, target.clone()).await;
                    }
                }
                _ => (),
            }
        }
    }

    pub async fn handle_bili_info(app_state: web::Data<AppState>, bv: &str, target: MsgTarget) {
        let params = BiliParams::new(bv).quality(true);
        let resp = app_state.http_services.get_bilibili_info(&params).await;
        if let Ok(bili_info) = resp {
            let msg = format!(
                "\n\n{}\n\nup主:    {}\n\n发布时间: {}\n\n播放量:    {}\n\n点赞: {} · 投币: {}\n\n收藏: {} · 分享: {}\n\n视频链接: {}",
                bili_info.title,
                bili_info.up,
                bili_info.date,
                bili_info.view,
                bili_info.like,
                bili_info.coin,
                bili_info.fav,
                bili_info.share,
                bili_info.video_url,
            );
            let current_dir = env::current_dir().unwrap();
            let face_path = current_dir
                .join(format!("../data/face/{}.jpg", bili_info.bv))
                .canonicalize()
                .unwrap()
                .display()
                .to_string();
            let video_path = current_dir
                .join(format!("../data/video/{}.mp4", bili_info.bv))
                .canonicalize()
                .unwrap()
                .display()
                .to_string();
            let message = target.clone().new_message().image(&face_path).text(&msg);
            let video = target.clone().new_message().video(&video_path);
            debug!("{:#}", json!(video));
            if let Ok(data) = app_state.http_services.send_message(message).await {
                let data = data.json::<Value>().await.unwrap();
                if data["status"].as_str().unwrap().eq("failed") {
                    error!("{} 发送视频信息失败", bili_info.bv)
                } else {
                    info!("{} 发送视频信息成功", bili_info.bv)
                }
            } else {
                error!("连接bot_server失败");
                return;
            }
            if let Ok(data) = app_state.http_services.send_message(video).await {
                let data = data.json::<Value>().await.unwrap();
                if data["status"].as_str().unwrap().eq("failed") {
                    error!("{} 发送视频失败", bili_info.bv)
                } else {
                    info!("{} 发送视频成功", bili_info.bv)
                }
            }
        } else {
            error!("获取{}信息失败\n{}", bv, resp.err().unwrap());
        }
    }

    pub async fn handle_competition_info(
        app_state: web::Data<AppState>,
        target: MsgTarget,
        args: Vec<String>,
    ) {
        let competitions = app_state
            .competitions
            .read()
            .await
            .clone()
            .into_iter()
            .filter(|competition| competition.start_time > Utc::now().timestamp())
            .collect::<Vec<Competition>>();
        if competitions.is_empty() {
            return;
        }
        if args.len() == 1 {
            let mut text = String::new();
            for i in competitions[..competitions.len()].iter() {
                let local_time = TimeConverter::from_utc_to_utc8(
                    &DateTime::from_timestamp(i.start_time, 0).unwrap(),
                );
                text += &format!(
                    "{}\n{}至{}\n{}\n\n",
                    i.name,
                    local_time.format("%Y/%m/%d-%H:%M"),
                    (local_time + Duration::from_secs(i.duration)).format("%Y/%m/%d-%H:%M"),
                    i.link
                );
            }
            let msg = target.new_message().text(text.strip_suffix("\n").unwrap());
            if let Ok(data) = app_state.http_services.send_message(msg).await {
                let data = data.json::<Value>().await.unwrap();
                if data["status"].as_str().unwrap().eq("failed") {
                    error!("发送比赛信息失败");
                } else {
                    info!("发送比赛信息成功");
                }
            } else {
                error!("连接bot_server失败");
                return;
            }
        }
    }
}

#[test]
fn test_cqmessage() {
    let message = MsgTarget::Group { group_id: 123456 }
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
