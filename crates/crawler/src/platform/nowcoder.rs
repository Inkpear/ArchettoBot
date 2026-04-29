use async_trait::async_trait;
use chrono::Utc;
use scraper::{Html, Selector};

use super::time_util::parse_utc8_timestamps;
use super::ContestFetcher;
use crate::error::Result;
use crate::models::Competition;

pub struct NowCoder;

#[async_trait]
impl ContestFetcher for NowCoder {
    fn platform_name(&self) -> &str {
        "NowCoder"
    }

    async fn fetch(&self) -> Result<Vec<Competition>> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .build()?;

        let html = client
            .get("https://ac.nowcoder.com/acm/contest/vip-index")
            .send()
            .await?
            .text()
            .await?;

        parse_nowcoder_html(&html)
    }
}

fn parse_nowcoder_html(html: &str) -> Result<Vec<Competition>> {
    let doc = Html::parse_document(html);
    let now = Utc::now().timestamp();
    let mut competitions = Vec::new();

    let item_sel = Selector::parse(".platform-mod .platform-item-cont, .contest-item").unwrap();
    let link_sel = Selector::parse("a").unwrap();

    for item in doc.select(&item_sel) {
        let link_elem = match item.select(&link_sel).next() {
            Some(e) => e,
            None => continue,
        };
        let href = link_elem.value().attr("href").unwrap_or("");
        let name: String = link_elem
            .text()
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_owned();
        let link = if href.starts_with("http") {
            href.to_owned()
        } else {
            format!("https://ac.nowcoder.com{}", href)
        };

        if name.is_empty() {
            continue;
        }

        let full_text: String = item.text().collect::<Vec<_>>().join(" ");
        let (start_time, duration) = parse_nowcoder_time(&full_text);

        if start_time > 0 && start_time + duration as i64 > now {
            competitions.push(Competition {
                link,
                name,
                start_time,
                duration,
                platform: "NowCoder".to_owned(),
                notified: false,
            });
        }
    }

    Ok(competitions)
}

fn parse_nowcoder_time(text: &str) -> (i64, i32) {
    // Prefer "比赛时间" over "报名时间". Page format:
    //   报名时间：2026-04-15 10:00 至 2026-05-01 17:00
    //   比赛时间：2026-05-01 12:00 至 2026-05-01 17:00
    let search_text = if let Some(pos) = text.find("比赛时间") {
        &text[pos..]
    } else {
        text
    };

    let parsed = parse_utc8_timestamps(search_text);

    if parsed.len() >= 2 {
        (parsed[0], ((parsed[1] - parsed[0]).max(0)) as i32)
    } else if let Some(&ts) = parsed.first() {
        (ts, 7200)
    } else {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono::Utc;

    #[test]
    fn parse_time_single() {
        // "2025-06-15 10:00" in UTC+8 → 2025-06-15 02:00 UTC
        let (ts, dur) = parse_nowcoder_time("比赛时间 2025-06-15 10:00 至 2025-06-15 12:00");
        let expected_utc = crate::UTC8
            .with_ymd_and_hms(2025, 6, 15, 10, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
            .timestamp();
        assert_eq!(ts, expected_utc);
        assert_eq!(dur, 7200);
    }

    #[test]
    fn parse_time_single_fallback() {
        // "2025-06-15 14:00" in UTC+8
        let (ts, dur) = parse_nowcoder_time("开始于 2025-06-15 14:00");
        let expected_utc = crate::UTC8
            .with_ymd_and_hms(2025, 6, 15, 14, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
            .timestamp();
        assert_eq!(ts, expected_utc);
        assert_eq!(dur, 7200);
    }

    #[test]
    fn parse_html_finds_contests() {
        let html = r#"
        <div class="platform-mod">
        <div class="platform-item-cont">
            <a href="/acm/contest/12345">牛客周赛 Round 100</a>
            <span>比赛时间 2030-06-15 19:00 至 2030-06-15 21:00</span>
        </div>
        <div class="platform-item-cont">
            <a href="/acm/contest/67890">牛客小白月赛</a>
            <span>比赛时间 2030-07-01 14:00 至 2030-07-01 17:00</span>
        </div>
        </div>
        "#;
        let result = parse_nowcoder_html(html).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "牛客周赛 Round 100");
        assert_eq!(result[0].platform, "NowCoder");
        assert_eq!(result[0].duration, 7200);
        assert!(result[0].link.contains("12345"));
    }

    #[test]
    fn parse_nowcoder_prefers_contest_time_over_registration_time() {
        // Real page: 报名时间 comes before 比赛时间, parser must prefer 比赛时间
        let (ts, dur) = parse_nowcoder_time(
            "报名时间：2026-04-15 10:00 至 2026-05-01 17:00 比赛时间：2026-05-01 12:00 至 2026-05-01 17:00",
        );
        assert_eq!(dur, 5 * 3600); // 5 hours from contest time, not 16 days from registration
        assert!(ts > 0);
    }

    #[test]
    fn parse_nowcoder_timezone_utc8_to_utc() {
        // "比赛时间 2026-05-01 12:00" in UTC+8 should be stored as 2026-05-01 04:00 UTC
        let (ts, _dur) = parse_nowcoder_time("比赛时间 2026-05-01 12:00 至 2026-05-01 17:00");
        let expected_utc = crate::UTC8
            .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
            .timestamp();
        assert_eq!(ts, expected_utc);
    }
}
