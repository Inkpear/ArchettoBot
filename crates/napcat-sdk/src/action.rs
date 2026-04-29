use serde::{Deserialize, Serialize};

use crate::message::{ForwardNode, Message};

#[derive(Debug, Clone, Serialize)]
pub struct ApiRequest {
    pub action: String,
    pub params: serde_json::Value,
    pub echo: String,
}

impl ApiRequest {
    pub fn new(action: &str, params: serde_json::Value) -> Self {
        Self {
            action: action.to_owned(),
            params,
            echo: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn send_group_msg(group_id: i64, message: &Message) -> Self {
        Self::new(
            "send_group_msg",
            serde_json::json!({
                "group_id": group_id,
                "message": message.0,
            }),
        )
    }

    pub fn send_private_msg(user_id: i64, message: &Message) -> Self {
        Self::new(
            "send_private_msg",
            serde_json::json!({
                "user_id": user_id,
                "message": message.0,
            }),
        )
    }

    pub fn delete_msg(message_id: i64) -> Self {
        Self::new(
            "delete_msg",
            serde_json::json!({
                "message_id": message_id,
            }),
        )
    }

    pub fn get_group_member_info(group_id: i64, user_id: i64) -> Self {
        Self::new(
            "get_group_member_info",
            serde_json::json!({
                "group_id": group_id,
                "user_id": user_id,
            }),
        )
    }

    pub fn get_friend_list() -> Self {
        Self::new("get_friend_list", serde_json::json!({}))
    }

    pub fn get_group_list() -> Self {
        Self::new("get_group_list", serde_json::json!({}))
    }

    pub fn send_group_forward_msg(group_id: i64, nodes: &[ForwardNode]) -> Self {
        Self::new(
            "send_group_forward_msg",
            serde_json::json!({
                "group_id": group_id.to_string(),
                "message": nodes,
            }),
        )
    }

    pub fn send_private_forward_msg(user_id: i64, nodes: &[ForwardNode]) -> Self {
        Self::new(
            "send_private_forward_msg",
            serde_json::json!({
                "user_id": user_id.to_string(),
                "message": nodes,
            }),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
    pub status: String,
    pub retcode: i64,
    pub data: serde_json::Value,
    pub echo: String,
}
