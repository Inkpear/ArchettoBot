use std::sync::Arc;

use crawler::models::Competition;
use log::info;
use napcat_sdk::Message;

use crate::db::Target;
use crate::util::send_to_target;
use crate::AppState;

pub async fn query_competitions(
    state: &Arc<AppState>,
    target: &Target,
    args: &str,
) -> anyhow::Result<()> {
    let limit: usize = match args.trim() {
        "all" | "全部" => 20,
        s => s.parse::<usize>().unwrap_or(5),
    };
    info!("query_competitions: limit={limit}, target={target:?}");

    let competitions = state.db.get_upcoming_competitions(limit).await?;
    info!(
        "query_competitions: got {} competitions",
        competitions.len()
    );

    if competitions.is_empty() {
        info!("query_competitions: sending empty message");
        let msg = Message::new().text("暂无即将开始的比赛");
        send_to_target(&state.nap, target, msg).await;
        return Ok(());
    }

    let text = format_competition_list(&competitions);
    info!("query_competitions: formatted text length={}", text.len());
    let msg = Message::new().text(&text);
    send_to_target(&state.nap, target, msg).await;

    Ok(())
}

fn format_time_utc8(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| {
            let utc8 = dt.with_timezone(&crawler::UTC8);
            utc8.format("%Y/%m/%d-%H:%M").to_string()
        })
        .unwrap_or_else(|| "未知".to_owned())
}

fn format_competition_list(competitions: &[Competition]) -> String {
    let mut text = String::new();
    for c in competitions {
        let start = format_time_utc8(c.start_time);
        let end = format_time_utc8(c.start_time + c.duration as i64);
        let hours = c.duration / 3600;
        let minutes = (c.duration % 3600) / 60;
        let duration_str = if minutes == 0 {
            format!("{}小时", hours)
        } else {
            format!("{}小时{}分", hours, minutes)
        };
        text.push_str(&format!(
            "{}\n{}至{}\n时长: {}\n{}\n\n",
            c.name, start, end, duration_str, c.link
        ));
    }
    text.truncate(text.trim_end().len());
    text
}
