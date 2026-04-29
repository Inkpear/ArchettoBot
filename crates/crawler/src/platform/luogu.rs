use async_trait::async_trait;
use chrono::Utc;
use scraper::{Html, Selector};

use super::time_util::parse_utc8_timestamps;
use super::ContestFetcher;
use crate::error::Result;
use crate::models::Competition;

pub struct Luogu;

#[async_trait]
impl ContestFetcher for Luogu {
    fn platform_name(&self) -> &str {
        "Luogu"
    }

    async fn fetch(&self) -> Result<Vec<Competition>> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0")
            .build()?;

        let html = client
            .get("https://www.luogu.com.cn/contest/list?page=1&_contentOnly=1")
            .header("x-requested-with", "XMLHttpRequest")
            .send()
            .await?
            .text()
            .await?;

        parse_luogu_html(&html)
    }
}

fn parse_luogu_html(html: &str) -> Result<Vec<Competition>> {
    let doc = Html::parse_document(html);
    let now = Utc::now().timestamp();
    let mut competitions = Vec::new();

    let row_sel = Selector::parse("tr[data-rid]").unwrap();
    let link_sel = Selector::parse("a").unwrap();

    for row in doc.select(&row_sel) {
        let link_elem = match row.select(&link_sel).next() {
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
        let link = format!("https://www.luogu.com.cn{}", href);

        if name.is_empty() {
            continue;
        }

        let full_text: String = row.text().collect::<Vec<_>>().join(" ");
        let (start_time, end_time) = parse_luogu_times(&full_text);

        // Luogu page displays UTC+8 local time
        if start_time > 0 && end_time > start_time && end_time > now {
            competitions.push(Competition {
                link,
                name,
                start_time,
                duration: (end_time - start_time).max(0) as i32,
                platform: "Luogu".to_owned(),
                notified: false,
            });
        }
    }

    Ok(competitions)
}

/// Parse start and end timestamps from Luogu row text.
/// Luogu times are in UTC+8. Returns UTC timestamps.
fn parse_luogu_times(text: &str) -> (i64, i64) {
    let parsed = parse_utc8_timestamps(text);
    if parsed.len() >= 2 {
        (parsed[0], parsed[1])
    } else if let Some(&ts) = parsed.first() {
        (ts, ts + 10800) // default 3h
    } else {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_times_from_text() {
        // "2025-06-15 10:00" UTC+8 → UTC
        let (ts, end) = parse_luogu_times("比赛时间 2025-06-15 10:00 至 2025-06-15 13:00");
        let expected_start = crate::UTC8
            .with_ymd_and_hms(2025, 6, 15, 10, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
            .timestamp();
        let expected_end = crate::UTC8
            .with_ymd_and_hms(2025, 6, 15, 13, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
            .timestamp();
        assert_eq!(ts, expected_start);
        assert_eq!(end, expected_end);
    }

    #[test]
    fn parse_html_finds_contests() {
        let html = r#"
        <table>
        <tbody>
        <tr data-rid="123">
            <td><a href="/contest/123456">【LGR-200】洛谷 Round 200</a></td>
            <td>2030-06-15 14:00</td>
            <td>2030-06-15 17:00</td>
        </tr>
        <tr data-rid="456">
            <td><a href="/contest/789012">洛谷月赛 2026</a></td>
            <td>2030-07-01 10:00</td>
            <td>2030-07-01 13:00</td>
        </tr>
        </tbody>
        </table>
        "#;
        let result = parse_luogu_html(html).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "【LGR-200】洛谷 Round 200");
        assert_eq!(result[0].platform, "Luogu");
        assert_eq!(result[0].duration, 10800); // 3h
        assert!(result[0].link.contains("123456"));
    }
}
