use log::{error, info};
use napcat_sdk::{ForwardNode, Message};

use crate::db::Target;

pub fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub async fn send_to_target(nap: &napcat_sdk::NapClient, target: &Target, msg: Message) {
    let result = match target {
        Target::Group { group_id } => nap.send_to_group(*group_id, msg).await,
        Target::Private { user_id } => nap.send_to_user(*user_id, msg).await,
    };
    match result {
        Ok(id) => info!("send_to_target OK: msg_id={id}"),
        Err(e) => error!("send_to_target failed for {target:?}: {e}"),
    }
}

pub async fn send_forward_to_target(
    nap: &napcat_sdk::NapClient,
    target: &Target,
    nodes: &[ForwardNode],
) {
    let result = match target {
        Target::Group { group_id } => nap.send_group_forward_msg(*group_id, nodes).await,
        Target::Private { user_id } => nap.send_private_forward_msg(*user_id, nodes).await,
    };
    match result {
        Ok(id) => info!("send_forward_to_target OK: msg_id={id}"),
        Err(e) => error!("send_forward_to_target failed for {target:?}: {e}"),
    }
}
