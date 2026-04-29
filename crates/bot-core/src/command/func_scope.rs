use std::sync::Arc;

use napcat_sdk::Message;

use crate::db::Target;
use crate::util::send_to_target;
use crate::AppState;

/// `target` — whose config to modify. `reply` — where to send the response.
pub async fn set_func(
    state: &Arc<AppState>,
    target: &Target,
    reply: &Target,
    func_key: &str,
    value: bool,
) -> anyhow::Result<()> {
    let valid_keys = ["bili_parse", "competition", "welcome"];
    let key = match func_key {
        "bilibili" | "B站" | "bili_parse" => "bili_parse",
        "比赛" | "competition" => "competition",
        "欢迎" | "welcome" | "通知" => "welcome",
        k if valid_keys.contains(&k) => k,
        _ => {
            let msg = Message::new().text("未知功能。可用: bili_parse / competition / welcome");
            send_to_target(&state.nap, reply, msg).await;
            return Ok(());
        }
    };

    state
        .db
        .set_func_scope(target.target_type(), target.target_id(), key, value)
        .await?;

    let msg = Message::new().text(&format!(
        "{} → {}",
        key,
        if value { "开启" } else { "关闭" }
    ));
    send_to_target(&state.nap, reply, msg).await;
    Ok(())
}

/// `target` — which group to set welcome for. `reply` — where to send the response.
pub async fn set_welcome(
    state: &Arc<AppState>,
    target: &Target,
    reply: &Target,
    message: &str,
) -> anyhow::Result<()> {
    let group_id = match target {
        Target::Group { group_id } => *group_id,
        Target::Private { .. } => {
            let msg = Message::new().text("入群欢迎仅可在群聊中设置");
            send_to_target(&state.nap, reply, msg).await;
            return Ok(());
        }
    };

    state.db.set_welcome_message(group_id, message).await?;
    let msg = Message::new().text("入群欢迎词已更新");
    send_to_target(&state.nap, reply, msg).await;
    Ok(())
}

/// `target` — whose status to show. `reply` — where to send the response.
pub async fn query_status(
    state: &Arc<AppState>,
    target: &Target,
    reply: &Target,
) -> anyhow::Result<()> {
    let scope = state
        .db
        .get_func_scope(target.target_type(), target.target_id())
        .await?;

    let is_group = matches!(target, Target::Group { .. });
    let target_label = match target {
        Target::Group { group_id } => format!("群 {}", group_id),
        Target::Private { user_id } => format!("私聊 {}", user_id),
    };

    let bili = yn(scope.bili_parse);
    let comp = yn(scope.competition);
    let welc = if is_group {
        yn(scope.welcome).to_owned()
    } else {
        "N/A（仅群聊可用）".to_owned()
    };

    let text = format!(
        "当前配置 ({target}):\n  B站解析: {bili}\n  比赛通知: {comp}\n  入群欢迎: {welc}",
        target = target_label,
        bili = bili,
        comp = comp,
        welc = welc,
    );
    let msg = Message::new().text(&text);
    send_to_target(&state.nap, reply, msg).await;
    Ok(())
}

fn yn(v: bool) -> &'static str {
    if v {
        "开启"
    } else {
        "关闭"
    }
}
