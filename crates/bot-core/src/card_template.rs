use chrono::Utc;
use crawler::models::Competition;

use crate::card_gen::RenderManager;
use crate::util::escape_html;

struct PlatformTheme {
    primary: &'static str,
    primary_dark: &'static str,
    bg: &'static str,
    card_bg: &'static str,
    tag_bg: &'static str,
    tag_text: &'static str,
    header_text: &'static str,
    text: &'static str,
    text_secondary: &'static str,
}

fn platform_theme(platform: &str) -> PlatformTheme {
    match platform {
        "LeetCode" => PlatformTheme {
            primary: "#FFA116",
            primary_dark: "#CC8112",
            bg: "#FFF8F0",
            card_bg: "#FFFFFF",
            tag_bg: "#FFF2E0",
            tag_text: "#B36800",
            header_text: "#1A1A1A",
            text: "#1E293B",
            text_secondary: "#64748B",
        },
        "NowCoder" => PlatformTheme {
            primary: "#25C26E",
            primary_dark: "#1DA85A",
            bg: "#F0FDF6",
            card_bg: "#FFFFFF",
            tag_bg: "#DCFCE7",
            tag_text: "#166534",
            header_text: "#FFFFFF",
            text: "#1E293B",
            text_secondary: "#64748B",
        },
        "Codeforces" => PlatformTheme {
            primary: "#B81F1F",
            primary_dark: "#8B0000",
            bg: "#FEF5F5",
            card_bg: "#FFFFFF",
            tag_bg: "#FEE2E2",
            tag_text: "#991B1B",
            header_text: "#FFFFFF",
            text: "#1E293B",
            text_secondary: "#64748B",
        },
        "AtCoder" => PlatformTheme {
            primary: "#0077B6",
            primary_dark: "#005C8A",
            bg: "#F0F7FC",
            card_bg: "#FFFFFF",
            tag_bg: "#DBEAFE",
            tag_text: "#1E40AF",
            header_text: "#FFFFFF",
            text: "#1E293B",
            text_secondary: "#64748B",
        },
        "Luogu" => PlatformTheme {
            primary: "#5B21B6",
            primary_dark: "#4C1D95",
            bg: "#F8F5FF",
            card_bg: "#FFFFFF",
            tag_bg: "#EDE9FE",
            tag_text: "#5B21B6",
            header_text: "#FFFFFF",
            text: "#1E293B",
            text_secondary: "#64748B",
        },
        _ => PlatformTheme {
            primary: "#3B82F6",
            primary_dark: "#2563EB",
            bg: "#F0F6FF",
            card_bg: "#FFFFFF",
            tag_bg: "#DBEAFE",
            tag_text: "#1E40AF",
            header_text: "#FFFFFF",
            text: "#1E293B",
            text_secondary: "#64748B",
        },
    }
}

pub fn competition_card_html(c: &Competition) -> String {
    let theme = platform_theme(&c.platform);

    let start_dt = chrono::DateTime::from_timestamp(c.start_time, 0)
        .map(|dt| dt.with_timezone(&crawler::UTC8));
    let end_dt = chrono::DateTime::from_timestamp(c.start_time + c.duration as i64, 0)
        .map(|dt| dt.with_timezone(&crawler::UTC8));

    let start_date = start_dt
        .map(|d| d.format("%m/%d").to_string())
        .unwrap_or_default();
    let start_time = start_dt
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default();
    let end_date = end_dt
        .map(|d| d.format("%m/%d").to_string())
        .unwrap_or_default();
    let end_time = end_dt
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default();

    let hours = c.duration / 3600;
    let minutes = (c.duration % 3600) / 60;
    let duration_str = if minutes == 0 {
        format!("{} 小时", hours)
    } else {
        format!("{} 小时 {} 分", hours, minutes)
    };

    let part_icon = c
        .platform
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    format!(
        r#"<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"><style>
        *{{margin:0;padding:0;box-sizing:border-box;}}
        body{{font-family:"Noto Sans CJK SC","PingFang SC","Microsoft YaHei",sans-serif;-webkit-font-smoothing:antialiased;background:transparent;width:800px;height:600px;}}
        .card{{width:800px;height:600px;border-radius:16px;overflow:hidden;display:flex;flex-direction:column;background:{card_bg};}}
        .header{{background:linear-gradient(135deg,{primary} 0%,{primary_dark} 100%);padding:22px 32px;display:flex;align-items:center;gap:14px;flex-shrink:0;}}
        .header-icon{{width:44px;height:44px;border-radius:12px;background:rgba(255,255,255,0.25);display:flex;align-items:center;justify-content:center;font-size:22px;font-weight:800;color:{header_text};flex-shrink:0;}}
        .header-name{{font-size:28px;font-weight:700;color:{header_text};letter-spacing:-0.5px;}}
        .header-tag{{margin-left:auto;background:{tag_bg};color:{tag_text};padding:6px 14px;border-radius:8px;font-size:18px;font-weight:600;}}
        .body{{padding:28px 32px;display:flex;flex-direction:column;flex-grow:1;}}
        .title{{font-size:36px;font-weight:700;color:{text};line-height:1.35;margin-bottom:24px;}}
        .upcoming{{display:inline-block;background:{tag_bg};color:{tag_text};padding:4px 16px;border-radius:6px;font-size:18px;font-weight:600;margin-top:auto;margin-bottom:12px;}}
        .info-row{{display:flex;gap:16px;}}
        .info-card{{flex:1;background:{bg};border-radius:12px;padding:18px 20px;border-left:4px solid {primary};}}
        .info-label{{font-size:16px;color:{text_secondary};margin-bottom:6px;}}
        .info-date{{font-size:22px;font-weight:700;color:{text};}}
        .info-time{{font-size:18px;color:{text_secondary};margin-top:2px;}}
        .hint{{margin-top:20px;text-align:center;font-size:16px;color:{text_secondary};border-top:1px solid #E2E8F0;padding-top:14px;}}
        </style></head><body>
        <div class="card">
            <div class="header">
                <div class="header-icon">{part_icon}</div>
                <div class="header-name">{platform}</div>
            </div>
            <div class="body">
                <div class="title">{title}</div>
                <div class="upcoming">即将开始</div>
                <div class="info-row">
                    <div class="info-card">
                        <div class="info-label">开始时间</div>
                        <div class="info-date">{start_date}</div>
                        <div class="info-time">{start_time} UTC+8</div>
                    </div>
                    <div class="info-card">
                        <div class="info-label">结束时间</div>
                        <div class="info-date">{end_date}</div>
                        <div class="info-time">{end_time} UTC+8</div>
                    </div>
                    <div class="info-card">
                        <div class="info-label">持续时长</div>
                        <div class="info-date">{duration}</div>
                        <div class="info-time">共计</div>
                    </div>
                </div>
                <div class="hint">比赛链接请查看下方文字消息</div>
            </div>
        </div>
        </body></html>"#,
        bg = theme.bg,
        card_bg = theme.card_bg,
        primary = theme.primary,
        primary_dark = theme.primary_dark,
        tag_bg = theme.tag_bg,
        tag_text = theme.tag_text,
        header_text = theme.header_text,
        text = theme.text,
        text_secondary = theme.text_secondary,
        part_icon = part_icon,
        platform = escape_html(&c.platform),
        title = escape_html(&c.name),
        start_date = start_date,
        start_time = start_time,
        end_date = end_date,
        end_time = end_time,
        duration = duration_str,
    )
}

pub async fn generate_test_cards() -> anyhow::Result<()> {
    let renderer = RenderManager::new();
    let now = Utc::now().timestamp();

    let platforms = vec![
        ("LeetCode", "LeetCode 第 456 场周赛", 7200),
        ("NowCoder", "牛客练习赛 128", 5400),
        (
            "Codeforces",
            "Codeforces Round #998 (Div. 1 + Div. 2)",
            9000,
        ),
        ("AtCoder", "AtCoder Beginner Contest 399", 6000),
        ("Luogu", "洛谷 2026 春令营 · 省选模拟赛 Round 3", 10800),
    ];

    std::fs::create_dir_all("data/test-cards")?;

    for (platform, name, duration) in &platforms {
        let c = Competition {
            link: "https://example.com/contest".into(),
            name: name.to_string(),
            start_time: now + 86400,
            duration: *duration,
            platform: platform.to_string(),
            notified: false,
        };

        let html = competition_card_html(&c);
        let png_b64 = renderer
            .render(html, 800, 600)
            .await
            .map_err(|e| anyhow::anyhow!("Render {}: {}", platform, e))?;

        let png_bytes = base64_decode(&png_b64)
            .ok_or_else(|| anyhow::anyhow!("Failed to decode base64 for {}", platform))?;

        let path = format!("data/test-cards/{}.png", platform.to_lowercase());
        std::fs::write(&path, &png_bytes)?;
        println!("Generated: {}", path);
    }

    println!("Done! Check data/test-cards/");
    Ok(())
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let s = s.trim();
    let mut result = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &b in s.as_bytes() {
        if b == b'=' {
            break;
        }
        let val = TABLE.iter().position(|&c| c == b)? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(result)
}
