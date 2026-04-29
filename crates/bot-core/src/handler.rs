use std::sync::Arc;

use log::{debug, info, warn};
use napcat_sdk::{Message, MessageEvent, NoticeEvent};

use crate::command::{admin, bilibili, competition, func_scope};
use crate::db::Target;
use crate::util::send_to_target;
use crate::AppState;

pub fn register_handlers(state: &Arc<AppState>) {
    let s = Arc::clone(state);

    state.nap.on_message(move |event| {
        let state = Arc::clone(&s);
        tokio::spawn(async move {
            if let Err(e) = handle_message(&state, event).await {
                log::error!("Handler error: {}", e);
            }
        });
    });

    let s2 = Arc::clone(state);
    state.nap.on_notice(move |event| {
        let state = Arc::clone(&s2);
        tokio::spawn(async move {
            if let Err(e) = handle_notice(&state, event).await {
                log::error!("Notice handler error: {}", e);
            }
        });
    });
}

async fn handle_message(state: &Arc<AppState>, event: MessageEvent) -> anyhow::Result<()> {
    let raw = event.raw_message.trim().to_owned();
    let user_id = event.user_id;

    // Cache bot's own QQ for forward messages etc.
    {
        let mut bot_qq = state.bot_qq.write().await;
        if bot_qq.is_none() {
            *bot_qq = Some(event.self_id);
        }
    }

    let target = match event.group_id {
        Some(gid) => Target::Group { group_id: gid },
        None => Target::Private {
            user_id: event.user_id,
        },
    };
    info!("handle_message: user={user_id}, target={target:?}");

    let is_master = user_id == state.config.master;
    let is_admin = is_master || state.db.is_admin(user_id).await.unwrap_or(false);

    // Commands start with /
    if raw.starts_with('/') {
        let (cmd, args) = split_first(&raw[1..]);
        debug!("Command: cmd=\"{cmd}\", args=\"{args}\"");

        match cmd {
            "查比赛" => {
                let scope = state
                    .db
                    .get_func_scope(target.target_type(), target.target_id())
                    .await?;
                debug!("查比赛 check: competition={}", scope.competition);
                if scope.competition {
                    info!("Calling competition::query_competitions");
                    competition::query_competitions(state, &target, args).await?;
                } else {
                    debug!("查比赛 skipped: competition disabled for {:?}", target);
                }
            }

            "添加管理" if is_master => {
                let qq = parse_qq(args, &event);
                if let Some(qq) = qq {
                    admin::add_admin(state, &target, qq).await?;
                } else {
                    let msg = Message::new().text("用法: /添加管理 <QQ号|@某人>");
                    send_to_target(&state.nap, &target, msg).await;
                }
            }

            "删除管理" if is_master => {
                let qq = parse_qq(args, &event);
                if let Some(qq) = qq {
                    admin::remove_admin(state, &target, qq).await?;
                } else {
                    let msg = Message::new().text("用法: /删除管理 <QQ号|@某人>");
                    send_to_target(&state.nap, &target, msg).await;
                }
            }

            "config" if is_admin => {
                if args.trim().is_empty() {
                    let msg = Message::new().text(
                    "用法:\n/config status [-g <群号>|-p <QQ号>]\n/config <功能> <t|f> [-g <群号>|-p <QQ号>]\n功能: bili_parse / competition / welcome",
                );
                    send_to_target(&state.nap, &target, msg).await;
                    return Ok(());
                }

                let (target_override, remaining) = parse_target_flag(args);
                let effective_target = target_override.as_ref().unwrap_or(&target);

                // Validate explicit target (-g/-p) against live lists
                if target_override.is_some() {
                    if let Some(err_msg) = validate_target(state, effective_target).await {
                        let msg = Message::new().text(&err_msg);
                        send_to_target(&state.nap, &target, msg).await;
                        return Ok(());
                    }
                }

                let (sub_key, rest) = split_first(remaining);
                match sub_key {
                    "status" => {
                        // For explicit targets, the parse_target_flag already consumed -g/-p.
                        // But status might be used like `/config status -g 123`, so check rest too.
                        let (sub_override, _effective_rest) = parse_target_flag(rest);
                        let status_target = sub_override.as_ref().unwrap_or(effective_target);

                        if sub_override.is_some() {
                            if let Some(err_msg) = validate_target(state, status_target).await {
                                let msg = Message::new().text(&err_msg);
                                send_to_target(&state.nap, &target, msg).await;
                                return Ok(());
                            }
                        }

                        func_scope::query_status(state, status_target, &target).await?;
                    }
                    "" => {
                        let msg = Message::new().text(
                        "用法: /config <功能> <t|f> [-g <群号>|-p <QQ号>]\n功能: bili_parse / competition / welcome",
                    );
                        send_to_target(&state.nap, &target, msg).await;
                    }
                    "通知" | "welcome" => {
                        func_scope::set_welcome(state, effective_target, &target, rest).await?;
                    }
                    _ => {
                        let val = rest.trim().to_lowercase();
                        if val.is_empty() {
                            let msg = Message::new().text("用法: /config <功能> <t|f>\n功能: bili_parse / competition / welcome");
                            send_to_target(&state.nap, &target, msg).await;
                            return Ok(());
                        }
                        let enable = val == "t" || val == "true" || val == "1" || val == "开启";
                        func_scope::set_func(state, effective_target, &target, sub_key, enable)
                            .await?;
                    }
                }
            }

            "heart_beat" if is_master => {
                let val = args.trim().to_lowercase();
                let enable = val == "t" || val == "true" || val == "1" || val == "开启";
                state
                    .db
                    .set_setting("heart_beat_enabled", if enable { "true" } else { "false" })
                    .await?;
                if enable {
                    crate::schedule::start_heart_beat(state);
                } else {
                    crate::schedule::stop_heart_beat(state);
                }
                let msg =
                    Message::new().text(&format!("心跳已{}", if enable { "开启" } else { "关闭" }));
                send_to_target(&state.nap, &target, msg).await;
            }

            _ => {
                info!("Unknown command: /{}", cmd);
            }
        }

        return Ok(());
    }

    // B站 link parsing (no command prefix, triggered by BV号)
    if raw.contains("BV") || raw.contains("b23.tv") {
        let scope = state
            .db
            .get_func_scope(target.target_type(), target.target_id())
            .await?;
        debug!("B站 parse check: bili_parse={}", scope.bili_parse);
        if scope.bili_parse {
            info!("Calling bilibili::parse_bilibili for {}", raw);
            bilibili::parse_bilibili(state, &target, &raw).await?;
        } else {
            debug!("B站 parse skipped: bili_parse disabled for {:?}", target);
        }
        return Ok(());
    }

    debug!("Not a command, ignoring");
    Ok(())
}

async fn handle_notice(state: &Arc<AppState>, event: NoticeEvent) -> anyhow::Result<()> {
    use napcat_sdk::NoticeType;

    if let NoticeType::GroupIncrease = event.notice_type {
        if let Some(group_id) = event.group_id {
            let scope = state.db.get_func_scope("group", group_id).await?;
            if scope.welcome {
                if let Some(msg_text) = state.db.get_welcome_message(group_id).await? {
                    if let Some(user_id) = event.user_id {
                        let msg = Message::new()
                            .at(&user_id.to_string())
                            .text(&format!(" {}", msg_text));
                        if let Err(e) = state.nap.send_to_group(group_id, msg).await {
                            log::warn!("Welcome message send failed for group {}: {}", group_id, e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn split_first(s: &str) -> (&str, &str) {
    let s = s.trim();
    match s.find(char::is_whitespace) {
        Some(pos) => (&s[..pos], s[pos..].trim()),
        None => (s, ""),
    }
}

/// Validate that an explicitly specified target (-g/-p) exists in bot's live lists.
/// Returns Some(error_message) if invalid, None if valid or API unavailable.
async fn validate_target(state: &Arc<AppState>, target: &Target) -> Option<String> {
    match target {
        Target::Group { group_id } => match state.nap.get_group_list().await {
            Ok(groups) => {
                if !groups.iter().any(|g| g.group_id == *group_id) {
                    Some(format!("群 {} 不在机器人群列表中", group_id))
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("Failed to fetch group list for validation: {e}");
                None // Allow on API error — don't block config
            }
        },
        Target::Private { user_id } => match state.nap.get_friend_list().await {
            Ok(friends) => {
                if !friends.iter().any(|f| f.user_id == *user_id) {
                    Some(format!("用户 {} 不在机器人好友列表中", user_id))
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("Failed to fetch friend list for validation: {e}");
                None
            }
        },
    }
}

/// Parse optional `-g <group_id>` or `-p <user_id>` from args.
/// Returns (optional target override, remaining args with the flag stripped).
fn parse_target_flag(args: &str) -> (Option<Target>, &str) {
    let trimmed = args.trim();
    if let Some(rest) = trimmed.strip_prefix("-g ") {
        if let Some((id_str, rest)) = rest.split_once(char::is_whitespace) {
            if let Ok(gid) = id_str.parse::<i64>() {
                return (Some(Target::Group { group_id: gid }), rest);
            }
        }
    } else if let Some(rest) = trimmed.strip_prefix("-p ") {
        if let Some((id_str, rest)) = rest.split_once(char::is_whitespace) {
            if let Ok(uid) = id_str.parse::<i64>() {
                return (Some(Target::Private { user_id: uid }), rest);
            }
        }
    }
    (None, args)
}

fn parse_qq(args: &str, _event: &MessageEvent) -> Option<i64> {
    let args = args.trim();
    // Try @ mention: [CQ:at,qq=123456]
    if args.contains("[CQ:at,qq=") {
        if let Some(start) = args.find("qq=") {
            let rest = &args[start + 3..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            return rest[..end].parse().ok();
        }
    }
    // Try plain number
    args.parse().ok()
}
