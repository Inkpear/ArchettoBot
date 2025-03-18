use std::{char::ToLowercase, env, str::FromStr, time::Duration};

use actix_rt::time;
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

    pub fn get_id(&self) -> u64 {
        match self {
            MsgTarget::Group { group_id } => *group_id,
            MsgTarget::Private { user_id } => *user_id,
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
                "friend_recall" => MessageType::FriendRecall,
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
                let target = MsgTarget::new_group(group_id);
                if app_state
                    .func_scope_services
                    .get_value(&target)
                    .group_increase_welcome
                {
                    let msg = target.new_message().at(&user_id.to_string()).text(
                        app_state
                            .group_data
                            .read()
                            .await
                            .get_welcome_message(group_id)
                            .unwrap_or(&" 欢迎入群!".to_string()),
                    );
                    let _ = app_state.http_services.send_message(msg).await;
                }
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

        for i in messages.messages.iter() {
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

                    if args[0].eq("查询比赛") && func_scope.competition {
                        Self::handle_competition_info(
                            app_state.clone(),
                            target.clone(),
                            args.clone(),
                        )
                        .await;
                        return;
                    }

                    if args[0].eq("添加管理") && app_state.check_master(user_id).await {
                        Self::handle_add_admin(app_state, args, target, &messages, user_id).await;
                        return;
                    }

                    if args[0].eq("删除管理") && app_state.check_master(user_id).await {
                        Self::handle_delete_admin(app_state, args, target, &messages, user_id)
                            .await;
                        return;
                    }

                    if args[0].eq("set_config")
                        && (app_state.check_master(user_id).await
                            || app_state.check_admin(user_id).await)
                    {
                        Self::handle_set_config(app_state, args, target, user_id).await;
                        return;
                    }

                    if args[0].eq("heat_beat") && app_state.check_master(user_id).await {
                        let status = if let Some(status) = args.get(1) {
                            match status.to_lowercase().as_str() {
                                "t" | "true" => true,
                                "f" | "false" => false,
                                _ => {
                                    return;
                                }
                            }
                        } else {
                            return;
                        };
                        {
                            let mut config = app_state.config.write().await;
                            config.heart_beat.0 = status;
                            let _ = config.save();
                        }
                        let msg = target
                            .new_message()
                            .text(&format!("已设置心跳事件状态: {}", status));
                        let _ = app_state.http_services.send_message(msg).await;
                        return;
                    }
                }
                "json" => {
                    let value = serde_json::from_str::<Value>(
                        &serde_json::from_value::<String>(i.data.get("data").unwrap().clone())
                            .unwrap(),
                    );
                    if let Err(_) = value {
                        return;
                    }
                    let value = value.unwrap();
                    let title = value["meta"]["detail_1"]["title"].as_str();
                    if let None = title {
                        return;
                    }
                    let title = title.unwrap();
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
            if let Err(e) = app_state.http_services.send_message(message).await {
                error!("连接bot_server失败 {}", e);
                return;
            }
            let _ = app_state.http_services.send_message(video).await;
            info!("解析{}完成!", bv);
        } else {
            error!("获取{}信息失败\n{}", bv, resp.err().unwrap());
        }
    }

    pub async fn handle_competition_info(
        app_state: web::Data<AppState>,
        target: MsgTarget,
        args: Vec<String>,
    ) {
        let competitions = app_state.competitions.read().await.clone();
        let mut size = args
            .get(1)
            .unwrap_or(&"3".to_string())
            .parse::<usize>()
            .unwrap_or(3);
        if let Some(arg) = args.get(1) {
            size = if arg.eq("all") {
                competitions.len()
            } else {
                size
            };
        }

        let competitions = competitions
            .into_iter()
            .take(size)
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
            let msg = target
                .new_message()
                .text(text.strip_suffix("\n\n").unwrap());
            if let Err(e) = app_state.http_services.send_message(msg).await {
                error!("连接bot_server失败 {}", e);
                return;
            }
        }
    }

    pub async fn handle_add_admin(
        app_state: web::Data<AppState>,
        args: Vec<String>,
        target: MsgTarget,
        messages: &CqMessage,
        user_id: u64,
    ) {
        if let MsgTarget::Private { user_id: _ } = &target {
            if args.len() < 2 {
                let msg = target.new_message().text("缺少参数!");
                let res = app_state.http_services.send_message(msg).await;
                if let Err(e) = res {
                    error!("连接bot_server失败! {}", e);
                }
                return;
            }
            let new_admin_id = args[1].parse::<u64>();
            if let Err(_) = new_admin_id {
                let msg = target.new_message().text("请发送正确的qq号");
                let _ = app_state.http_services.send_message(msg).await;
                return;
            }
            let new_admin_id = new_admin_id.unwrap();
            app_state.user_config.write().await.add_admin(new_admin_id);
            info!("已添加管理 {}", new_admin_id);
            let reply = target
                .new_message()
                .text(&format!("成功添加管理: {}", new_admin_id));
            let _ = app_state.http_services.send_message(reply).await;

            return;
        }
        let at = messages.messages.iter().find(|item| item.type_.eq("at"));

        if let None = at {
            let msg = target
                .new_message()
                .at(&user_id.to_string())
                .text(" 缺少参数!");
            let res = app_state.http_services.send_message(msg).await;
            if let Err(e) = res {
                error!("连接bot_server失败! {}", e);
            }
            return;
        }
        debug!("{:#}", at.unwrap().data);
        let new_admin_id = at
            .unwrap()
            .data
            .get("qq")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();

        app_state.user_config.write().await.add_admin(new_admin_id);
        info!("已添加管理 {}", new_admin_id);
        let reply = target
            .new_message()
            .at(&user_id.to_string())
            .text(&format!(" 成功添加管理: {}", new_admin_id));
        let _ = app_state.http_services.send_message(reply).await;

        return;
    }

    pub async fn handle_delete_admin(
        app_state: web::Data<AppState>,
        args: Vec<String>,
        target: MsgTarget,
        messages: &CqMessage,
        user_id: u64,
    ) {
        if let MsgTarget::Private { user_id: _ } = &target {
            if args.len() < 2 {
                let msg = target.new_message().text("缺少参数!");
                let res = app_state.http_services.send_message(msg).await;
                if let Err(e) = res {
                    error!("连接bot_server失败! {}", e);
                }
                return;
            }
            let new_admin_id = args[1].parse::<u64>();
            if let Err(_) = new_admin_id {
                let msg = target.new_message().text("请发送正确的qq号");
                let _ = app_state.http_services.send_message(msg).await;
                return;
            }
            let new_admin_id = new_admin_id.unwrap();
            app_state
                .user_config
                .write()
                .await
                .delet_admin(new_admin_id);
            info!("已删除管理 {}", new_admin_id);
            let reply = target
                .new_message()
                .text(&format!("成功删除管理: {}", new_admin_id));
            let _ = app_state.http_services.send_message(reply).await;

            return;
        }
        let at = messages.messages.iter().find(|item| item.type_.eq("at"));

        if let None = at {
            let msg = target
                .new_message()
                .at(&user_id.to_string())
                .text(" 缺少参数!");
            let res = app_state.http_services.send_message(msg).await;
            if let Err(e) = res {
                error!("连接bot_server失败! {}", e);
            }
            return;
        }
        let new_admin_id = at
            .unwrap()
            .data
            .get("qq")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();

        if !app_state
            .user_config
            .write()
            .await
            .delet_admin(new_admin_id)
        {
            let reply = target
                .new_message()
                .at(&user_id.to_string())
                .text(&format!(" 不存在的bot管理员: {}", new_admin_id));
            let _ = app_state.http_services.send_message(reply).await;
            return;
        }
        info!("已删除管理 {}", new_admin_id);
        let reply = target
            .new_message()
            .at(&user_id.to_string())
            .text(&format!(" 成功删除管理: {}", new_admin_id));
        let _ = app_state.http_services.send_message(reply).await;

        return;
    }

    pub async fn handle_set_config(
        app_state: web::Data<AppState>,
        args: Vec<String>,
        target: MsgTarget,
        user_id: u64,
    ) {
        if let MsgTarget::Private { user_id: _ } = &target {
            return;
        }
        match args.get(1).zip(args.get(2)) {
            None => {
                let msg = target
                    .new_message()
                    .at(user_id.to_string().as_ref())
                    .text(" 缺少必要参数!");
                let _ = app_state.http_services.send_message(msg).await;
                return;
            }
            Some((action_type, action_status)) => {
                let action = match action_type.as_str() {
                    "哔哩哔哩视频解析" | "bv_parse" => "bili_parse",
                    "入群通知" | "迎新" => "group_increase_welcome",
                    "比赛通知" | "竞赛通知" => "competition",
                    "通知" => {
                        let group_id = target.get_id();
                        let mut welcom_msg = String::new();
                        for i in args[2..].iter() {
                            welcom_msg += i;
                        }
                        app_state
                            .group_data
                            .write()
                            .await
                            .set_welcome_message(group_id, &(" ".to_string() + &welcom_msg));
                        let msg = target
                            .new_message()
                            .at(&user_id.to_string())
                            .text(&format!(" 已设置入群通知为: {}", welcom_msg));
                        let _ = app_state.http_services.send_message(msg).await;
                        return;
                    }
                    _ => {
                        let msg = target
                            .new_message()
                            .at(&user_id.to_string())
                            .text(" 错误的功能名称!");
                        let _ = app_state.http_services.send_message(msg).await;
                        return;
                    }
                };
                let action_status = match action_status.to_lowercase().as_str() {
                    "t" | "true" => true,
                    "f" | "false" => false,
                    _ => {
                        let msg = target
                            .new_message()
                            .at(&user_id.to_string())
                            .text(" 未知的状态标识!");
                        let _ = app_state.http_services.send_message(msg).await;
                        return;
                    }
                };
                app_state
                    .func_scope_services
                    .set_scope(action, action_status, &target);
                let msg = target
                    .new_message()
                    .at(user_id.to_string().as_str())
                    .text(" 修改配置成功!");
                let _ = app_state.http_services.send_message(msg).await;
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
