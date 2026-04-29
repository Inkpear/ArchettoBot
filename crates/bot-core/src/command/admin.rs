use std::sync::Arc;

use napcat_sdk::Message;

use crate::db::Target;
use crate::util::send_to_target;
use crate::AppState;

pub async fn add_admin(state: &Arc<AppState>, target: &Target, qq: i64) -> anyhow::Result<()> {
    let changed = state.db.add_admin(qq).await?;
    let msg = if changed {
        Message::new().text(&format!("已添加管理员: {}", qq))
    } else {
        Message::new().text(&format!("{} 已经是管理员", qq))
    };
    send_to_target(&state.nap, target, msg).await;
    Ok(())
}

pub async fn remove_admin(state: &Arc<AppState>, target: &Target, qq: i64) -> anyhow::Result<()> {
    let changed = state.db.remove_admin(qq).await?;
    let msg = if changed {
        Message::new().text(&format!("已移除管理员: {}", qq))
    } else {
        Message::new().text(&format!("{} 不是管理员", qq))
    };
    send_to_target(&state.nap, target, msg).await;
    Ok(())
}
