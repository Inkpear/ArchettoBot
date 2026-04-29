use std::sync::Arc;
use std::time::Duration;

use chrono::{Timelike, Utc};
use log::{debug, error, info, warn};
use napcat_sdk::Message;
use tokio::time::timeout;

use crate::card_template::competition_card_html;
use crate::db::Target;
use crate::scheduler::Scheduler;
use crate::AppState;

pub async fn spawn_scheduled_tasks(state: &Arc<AppState>) {
    info!("Initializing scheduled tasks");
    let s = &state.scheduler;

    // Daily competition update at 2:00 AM (self-scheduling)
    fn schedule_competition_update(s: &Scheduler, state: &Arc<AppState>) {
        let next = next_daily_time(2, 0);
        let state = Arc::clone(state);
        s.once("competition_update", next, async move {
            match crawler::get_all_competitions().await {
                Ok(list) => {
                    if let Err(e) = state.db.upsert_competitions(&list).await {
                        error!("Competition upsert failed: {}", e);
                    }
                    if let Err(e) = state.db.clean_expired().await {
                        error!("Daily clean_expired failed: {}", e);
                    }
                    clean_invalid_targets(&state).await;
                    info!("Competition update done, {} contests", list.len());
                }
                Err(e) => error!("Competition fetch failed: {}", e),
            }
            schedule_competition_update(&state.scheduler, &state);
        });
    }
    schedule_competition_update(s, state);

    // Every 5 minutes: check for pending notifications
    {
        let state = Arc::clone(state);
        s.every("competition_notify", 300, move || {
            let state = Arc::clone(&state);
            async move {
                if !state.nap.is_connected().await {
                    info!("Notification check: NapCat not connected, skipping");
                    return;
                }
                info!("Notification check: starting");
                let pending = match state.db.get_pending_notifications().await {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Notification check failed: {}", e);
                        return;
                    }
                };
                let targets = match state.db.get_targets_with("competition").await {
                    Ok(t) => t,
                    Err(e) => {
                        error!("Get notification targets failed: {}", e);
                        return;
                    }
                };
                if pending.is_empty() {
                    debug!("No pending competitions");
                    return;
                }
                if targets.is_empty() {
                    info!("Notification check: no targets");
                    return;
                }

                debug!(
                    "Notification check: {} pending, {} targets",
                    pending.len(),
                    targets.len()
                );
                for c in &pending {
                    let html = competition_card_html(c);
                    let render_fut = state.renderer.render(html, 800, 600);
                    match timeout(Duration::from_secs(30), render_fut).await {
                        Ok(Ok(card)) => {
                            let mut any_sent = false;
                            for t in &targets {
                                let card_msg = Message::new().base64_image(&card);
                                match t {
                                    Target::Group { group_id } => {
                                        match state
                                            .nap
                                            .send_to_group(*group_id, card_msg.clone())
                                            .await
                                        {
                                            Ok(_) => any_sent = true,
                                            Err(e) => warn!(
                                                "Notification card send failed for {:?}: {}",
                                                t, e
                                            ),
                                        }
                                    }
                                    Target::Private { user_id } => {
                                        match state.nap.send_to_user(*user_id, card_msg).await {
                                            Ok(_) => any_sent = true,
                                            Err(e) => warn!(
                                                "Notification card send failed for {:?}: {}",
                                                t, e
                                            ),
                                        }
                                    }
                                }
                                let link_msg =
                                    Message::new().text(&format!("比赛链接: {}", &c.link));
                                match t {
                                    Target::Group { group_id } => {
                                        match state.nap.send_to_group(*group_id, link_msg).await {
                                            Ok(_) => any_sent = true,
                                            Err(e) => warn!(
                                                "Notification link send failed for {:?}: {}",
                                                t, e
                                            ),
                                        }
                                    }
                                    Target::Private { user_id } => {
                                        match state.nap.send_to_user(*user_id, link_msg).await {
                                            Ok(_) => any_sent = true,
                                            Err(e) => warn!(
                                                "Notification link send failed for {:?}: {}",
                                                t, e
                                            ),
                                        }
                                    }
                                }
                            }
                            if any_sent {
                                if let Err(e) = state.db.mark_notified(&c.link).await {
                                    error!("Mark notified failed for {}: {}", c.link, e);
                                }
                            } else {
                                info!("Notification for {} skipped: no targets reached", c.name);
                            }
                        }
                        Ok(Err(e)) => {
                            error!("Render failed for {}: {}", c.name, e);
                        }
                        Err(_) => {
                            error!("Render timed out for {}", c.name);
                        }
                    }
                }
                info!("Notification check: done");
            }
        });
    }

    maybe_start_heart_beat(state).await;
}

async fn maybe_start_heart_beat(state: &Arc<AppState>) {
    let enabled = state
        .db
        .get_setting("heart_beat_enabled")
        .await
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(state.config.heart_beat.enabled);

    if enabled {
        start_heart_beat(state);
    }
}

pub(crate) fn start_heart_beat(state: &Arc<AppState>) {
    state.scheduler.cancel("heart_beat");

    let interval = state.config.heart_beat.interval_secs;
    let msg_content = state.config.heart_beat.message.clone();
    let nap = state.nap.clone();
    let master = state.config.master;

    state.scheduler.every("heart_beat", interval, move || {
        let msg = Message::new().text(&msg_content.clone());
        let nap = nap.clone();
        let master = master;
        async move {
            let _ = nap.send_to_user(master, msg).await;
        }
    });
}

pub(crate) fn stop_heart_beat(state: &Arc<AppState>) {
    state.scheduler.cancel("heart_beat");
}

fn next_daily_time(hour: u32, minute: u32) -> chrono::DateTime<Utc> {
    let now = Utc::now();
    let mut next = now
        .with_hour(hour)
        .unwrap()
        .with_minute(minute)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    if next <= now {
        next += chrono::Duration::days(1);
    }
    next
}

/// Remove func_scopes entries for groups/friends the bot no longer belongs to.
async fn clean_invalid_targets(state: &Arc<AppState>) {
    let targets = match state.db.get_all_func_scope_targets().await {
        Ok(t) => t,
        Err(e) => {
            error!("clean_invalid_targets: failed to fetch targets: {e}");
            return;
        }
    };
    if targets.is_empty() {
        return;
    }

    let groups = match state.nap.get_group_list().await {
        Ok(g) => g,
        Err(e) => {
            warn!("clean_invalid_targets: failed to fetch group list: {e}");
            return;
        }
    };
    let friends = match state.nap.get_friend_list().await {
        Ok(f) => f,
        Err(e) => {
            warn!("clean_invalid_targets: failed to fetch friend list: {e}");
            return;
        }
    };

    let mut cleaned = 0;
    for (tt, id) in &targets {
        let valid = match tt.as_str() {
            "group" => groups.iter().any(|g| g.group_id == *id),
            "private" => friends.iter().any(|f| f.user_id == *id),
            _ => true,
        };
        if !valid {
            match state.db.remove_func_scope(tt, *id).await {
                Ok(true) => {
                    info!("clean_invalid_targets: removed {} {}", tt, id);
                    cleaned += 1;
                }
                Ok(false) => {}
                Err(e) => error!(
                    "clean_invalid_targets: delete failed for {} {}: {}",
                    tt, id, e
                ),
            }
        }
    }
    if cleaned > 0 {
        info!("clean_invalid_targets: cleaned {} invalid targets", cleaned);
    }
}
