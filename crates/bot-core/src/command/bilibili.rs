use std::sync::Arc;

use log::{debug, error, info};
use napcat_sdk::{ForwardNode, Message};

use crate::card_gen::generate_qr_data_uri;
use crate::db::Target;
use crate::util::{base64_encode, escape_html, send_forward_to_target, send_to_target};
use crate::AppState;

/// Extract the first B站-relevant URL (b23.tv or bilibili.com) from a message.
/// Handles URLs embedded in CQ:json mini-program data.
fn extract_url(input: &str) -> Option<String> {
    let input = input.trim();

    // b23.tv first — may be nested inside CQ:json data with JSON escapes
    if let Some(pos) = input.find("b23.tv") {
        // Scan backwards for the URL scheme start (handles "https://" and "https:\\/\\/")
        let prefix = &input[..pos];
        let start = prefix
            .rfind("https://")
            .or_else(|| prefix.rfind("https:\\/\\/"))
            .or_else(|| prefix.rfind("http://"))
            .unwrap_or(pos);
        // Scan forward for the URL end (", &, or whitespace)
        let suffix = &input[pos..];
        let end = suffix
            .find(|c: char| c == '"' || c == '\'' || c.is_whitespace())
            .map(|i| pos + i)
            .unwrap_or(input.len());
        // Clean up JSON escapes and HTML entities from encoded contexts
        return Some(input[start..end].replace("\\/", "/").replace("&amp;", "&"));
    }

    // bilibili.com direct URLs
    if let Some(pos) = input.find("bilibili.com") {
        let prefix = &input[..pos];
        let start = prefix.rfind("https://").unwrap_or(pos);
        let suffix = &input[pos..];
        let end = suffix
            .find(|c: char| c == '"' || c == '\'' || c.is_whitespace())
            .map(|i| pos + i)
            .unwrap_or(input.len());
        return Some(input[start..end].replace("\\/", "/"));
    }

    // Generic http URL as fallback (e.g. plain b站 link in text message)
    if let Some(pos) = input.find("http") {
        let url_part = &input[pos..];
        let end = url_part
            .find(|c: char| c.is_whitespace())
            .unwrap_or(url_part.len());
        return Some(url_part[..end].to_owned());
    }

    None
}

/// Extract BV number from a B站 URL or raw BV string.
fn extract_bv(input: &str) -> Option<String> {
    let input = input.trim();
    if let Some(pos) = input.find("BV") {
        let bv_part = &input[pos..];
        let end = bv_part
            .find(|c: char| c.is_whitespace() || c == '/' || c == '?' || c == '&')
            .unwrap_or(bv_part.len());
        let bv = bv_part[..end].to_owned();
        if bv.len() >= 10 {
            return Some(bv);
        }
    }
    if input.contains("b23.tv") {
        return None;
    }
    None
}

/// Try to resolve a b23.tv short URL to a full B站 URL
async fn resolve_b23_url(url: &str) -> Option<String> {
    let url = url.trim();
    if !url.contains("b23.tv") {
        return Some(url.to_owned());
    }

    debug!("resolve_b23_url: resolving {url}");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Mozilla/5.0")
        .build()
        .ok()?;

    let resp = client.head(url).send().await.ok()?;
    let location = resp.headers().get("Location")?.to_str().ok()?;
    debug!("resolve_b23_url: redirected to {location}");
    Some(location.to_owned())
}

/// Download an image URL and return it as a base64 data URI.
async fn download_image_as_base64(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .referer(true)
        .build()
        .ok()?;
    let bytes = client.get(url).send().await.ok()?.bytes().await.ok()?;
    let mime = if url.contains(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    Some(format!("data:{};base64,{}", mime, base64_encode(&bytes)))
}

fn format_count(n: u64) -> String {
    if n >= 10_000 {
        format!("{:.1}万", n as f64 / 10_000.0)
    } else {
        n.to_string()
    }
}

pub async fn parse_bilibili(
    state: &Arc<AppState>,
    target: &Target,
    raw_message: &str,
) -> anyhow::Result<()> {
    debug!("parse_bilibili: raw=\"{raw_message}\"");

    let url = extract_url(raw_message);
    debug!("parse_bilibili: extracted url={url:?}");
    let resolved = match url.as_deref() {
        Some(u) => resolve_b23_url(u).await,
        None => None,
    };
    let search_text = resolved.as_deref().unwrap_or(raw_message);

    let bv = match extract_bv(search_text) {
        Some(bv) => {
            info!("parse_bilibili: extracted BV={bv}");
            bv
        }
        None => {
            debug!("parse_bilibili: no BV found in message");
            return Ok(());
        }
    };

    info!("parse_bilibili: fetching info for {bv}");
    let info = crawler::bilibili::get_bilibili_info(&bv).await?;
    info!("parse_bilibili: fetched title=\"{}\"", info.title);

    let video_url = format!("https://www.bilibili.com/video/{}", bv);
    let cover_b64 = download_image_as_base64(&info.cover_url).await;
    let qr_b64 = generate_qr_data_uri(&video_url);

    let html = bilibili_card_html(
        &info,
        &video_url,
        cover_b64.as_deref().unwrap_or(""),
        &qr_b64,
    );

    let bot_qq = state.bot_qq().await;

    match state.renderer.render(html, 1600, 900).await {
        Ok(b64) => {
            info!(
                "parse_bilibili: render OK, sending forward msg (card {} bytes)",
                b64.len()
            );
            let nodes = vec![
                ForwardNode::new(bot_qq, "ArchettoBot", Message::new().base64_image(&b64)),
                ForwardNode::new(
                    bot_qq,
                    "ArchettoBot",
                    Message::new().text(&format!("视频链接: {}", video_url)),
                ),
            ];
            send_forward_to_target(&state.nap, target, &nodes).await;
        }
        Err(e) => {
            error!("parse_bilibili: render failed: {e}");
            let msg = Message::new().text(&format!("B站解析失败: {}", e));
            send_to_target(&state.nap, target, msg).await;
        }
    }

    // Spawn background task to download and send the video
    let nap = state.nap.clone();
    let target = target.clone();
    let bv_clone = bv.clone();
    let cid = info.cid;
    tokio::spawn(async move {
        match download_and_send_video(&nap, &target, &bv_clone, cid).await {
            Ok(()) => info!("Video sent successfully for {bv_clone}"),
            Err(e) => error!("Video download/send failed for {bv_clone}: {e}"),
        }
    });

    Ok(())
}

async fn download_and_send_video(
    nap: &napcat_sdk::NapClient,
    target: &Target,
    bv: &str,
    cid: i64,
) -> anyhow::Result<()> {
    let output_path = format!("data/video/{bv}.mp4");

    // Check cache
    if std::path::Path::new(&output_path).exists() {
        info!("Video cache hit for {bv}, sending directly");
        let msg = Message::new().video(&format!("/data/video/{bv}.mp4"));
        send_to_target(nap, target, msg).await;
        return Ok(());
    }

    info!("Downloading video streams for {bv}");
    let urls = crawler::bilibili_video::get_video_urls(bv, cid, true).await?;
    info!("Video quality: {}, downloading...", urls.quality);

    // Ensure data directory exists
    std::fs::create_dir_all("data/video")?;

    let video_path = format!("data/video/{bv}_video.m4s");
    let audio_path = format!("data/video/{bv}_audio.m4s");

    // Download video and audio streams in parallel
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .referer(true)
        .build()?;

    let (video_result, audio_result) = tokio::join!(
        download_file(&client, &urls.video_url, &video_path),
        download_file(&client, &urls.audio_url, &audio_path),
    );
    video_result?;
    audio_result?;

    info!("Download complete for {bv}, merging with ffmpeg...");

    // Merge with ffmpeg
    let ffmpeg_status = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "16",
            "-i",
            &video_path,
            "-i",
            &audio_path,
            "-c:v",
            "copy",
            "-c:a",
            "copy",
            "-y",
            &output_path,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !ffmpeg_status.success() {
        anyhow::bail!("ffmpeg merge failed with status: {ffmpeg_status}");
    }

    // Clean up temp files
    let _ = std::fs::remove_file(&video_path);
    let _ = std::fs::remove_file(&audio_path);

    info!("Merge complete for {bv}, sending video");
    // Container-accessible path: /data/video/ mounted from host data/video/
    let msg = Message::new().video(&format!("/data/video/{}.mp4", bv));
    send_to_target(nap, target, msg).await;

    Ok(())
}

async fn download_file(client: &reqwest::Client, url: &str, path: &str) -> anyhow::Result<()> {
    let resp = client
        .get(url)
        .header("Referer", "https://www.bilibili.com")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("Download failed: HTTP {} for {}", resp.status(), url);
    }
    let bytes = resp.bytes().await?;
    if bytes.len() < 1024 {
        anyhow::bail!(
            "Download too small ({} bytes), likely blocked by CDN for {}",
            bytes.len(),
            url
        );
    }
    tokio::fs::write(path, &bytes).await?;
    info!("Downloaded {} bytes to {path}", bytes.len());
    Ok(())
}

fn bilibili_card_html(
    info: &crawler::models::BiliInfo,
    _video_url: &str,
    cover_b64: &str,
    qr_b64: &str,
) -> String {
    let cover_html = if cover_b64.is_empty() {
        r#"<div class="cover"><div class="cover-placeholder">暂无封面</div></div>"#.to_string()
    } else {
        format!(
            r#"<div class="cover">
            <img class="cover-bg" src="{}" alt="" />
            <img class="cover-fg" src="{}" alt="" />
        </div>"#,
            cover_b64, cover_b64
        )
    };

    let qr_html = if qr_b64.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="qr-code"><img src="{}" alt="视频链接二维码" /></div>"#,
            qr_b64
        )
    };

    format!(
        r#"<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"><style>
        :root{{--scale-factor:2;--base-width:800px;--base-height:450px;--base-cover-width:480px;--card-width:calc(var(--base-width) * var(--scale-factor));--card-height:calc(var(--base-height) * var(--scale-factor));--cover-width:calc(var(--base-cover-width) * var(--scale-factor));}}
        body{{margin:0;font-family:'Noto Sans CJK SC','PingFang SC','Microsoft YaHei',sans-serif;-webkit-font-smoothing:antialiased;}}
        .card{{width:var(--card-width);height:var(--card-height);display:flex;background:#ffffff;border-radius:calc(12px * var(--scale-factor));overflow:hidden;box-shadow:0 calc(8px * var(--scale-factor)) calc(24px * var(--scale-factor)) rgba(0,0,0,0.1);}}
        .cover{{width:var(--cover-width);height:100%;flex-shrink:0;position:relative;overflow:hidden;background:#222;display:flex;align-items:center;justify-content:center;}}
        .cover-bg{{position:absolute;top:0;left:0;width:100%;height:100%;object-fit:cover;filter:blur(calc(20px * var(--scale-factor))) brightness(0.8);transform:scale(1.1);}}
        .cover-fg{{position:relative;z-index:1;max-width:100%;max-height:100%;object-fit:contain;box-shadow:0 calc(5px * var(--scale-factor)) calc(15px * var(--scale-factor)) rgba(0,0,0,0.3);border-radius:calc(4px * var(--scale-factor));}}
        .info{{flex-grow:1;padding:calc(28px * var(--scale-factor));display:flex;flex-direction:column;justify-content:space-between;overflow:hidden;color:#ffffff;background-color:#fb7299;background-image:radial-gradient(circle at 100% 100%,#ffd8e5,#fb7299);}}
        .title{{font-size:calc(22px * var(--scale-factor));font-weight:700;color:#ffffff;line-height:1.4;margin:0 0 calc(16px * var(--scale-factor)) 0;max-height:calc(92px * var(--scale-factor));display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden;text-overflow:ellipsis;word-break:break-word;}}
        .up-info{{display:flex;align-items:center;margin-bottom:calc(20px * var(--scale-factor));font-size:calc(15px * var(--scale-factor));}}
        .up-info strong{{color:#ffffff;}}
        .stats{{display:grid;grid-template-columns:repeat(2,1fr);gap:calc(16px * var(--scale-factor)) calc(20px * var(--scale-factor));font-size:calc(15px * var(--scale-factor));}}
        .stats div{{display:flex;align-items:center;white-space:nowrap;}}
        .stats-label{{color:#ffe8ee;margin-left:calc(8px * var(--scale-factor));}}
        .stats-value{{font-weight:700;color:#ffffff;margin-left:calc(6px * var(--scale-factor));}}
        .footer{{display:flex;align-items:flex-end;justify-content:space-between;padding-top:calc(16px * var(--scale-factor));margin-top:auto;}}
        .footer-text{{font-size:calc(13px * var(--scale-factor));color:#ffe8ee;line-height:1.5;}}
        .footer-text p{{margin:0 0 calc(4px * var(--scale-factor)) 0;}}
        .qr-code img{{width:calc(90px * var(--scale-factor));height:calc(90px * var(--scale-factor));border-radius:calc(8px * var(--scale-factor));background:#fff;padding:calc(4px * var(--scale-factor));}}
        </style></head><body>
        <div class="card">
            {cover_html}
            <div class="info">
                <div>
                    <h1 class="title">{title}</h1>
                    <div class="up-info"><span>UP主: <strong>{author}</strong></span></div>
                    <div class="stats">
                        <div>📺<span class="stats-label">播放</span><span class="stats-value">{play}</span></div>
                        <div>👍<span class="stats-label">点赞</span><span class="stats-value">{like}</span></div>
                        <div>🪙<span class="stats-label">投币</span><span class="stats-value">{coin}</span></div>
                        <div>⭐<span class="stats-label">收藏</span><span class="stats-value">{fav}</span></div>
                    </div>
                </div>
                <div class="footer">
                    <div class="footer-text">
                        <p>发布于: {date}</p>
                        <p>扫码可直接观看</p>
                    </div>
                    {qr_html}
                </div>
            </div>
        </div>
        </body></html>"#,
        cover_html = cover_html,
        title = escape_html(&info.title),
        author = escape_html(&info.author),
        play = format_count(info.play_count),
        like = format_count(info.like_count),
        coin = format_count(info.coin_count),
        fav = format_count(info.fav_count),
        date = info.publish_date,
        qr_html = qr_html,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_b23_from_json_cq() {
        // Simulated mini-program CQ:json message
        let raw = r#"[CQ:json,data={"ver":"1.0.0.19"&#44;"prompt":"&#91;QQ小程序&#93;test"&#44;"meta":{"detail_1":{"qqdocurl":"https:\/\/b23.tv\/9hhM3c6?share_medium=android&amp;share_source=qq&amp;bbid=XXX"}}}]"#;
        let url = extract_url(raw);
        assert_eq!(
            url.as_deref(),
            Some("https://b23.tv/9hhM3c6?share_medium=android&share_source=qq&bbid=XXX")
        );
    }

    #[test]
    fn extract_plain_b23_url() {
        let raw = "https://b23.tv/abc123 看看这个";
        let url = extract_url(raw);
        assert_eq!(url.as_deref(), Some("https://b23.tv/abc123"));
    }

    #[test]
    fn extract_bilibili_url() {
        let raw = "https://www.bilibili.com/video/BV1xx411c7mD?p=1";
        let url = extract_url(raw);
        assert_eq!(
            url.as_deref(),
            Some("https://www.bilibili.com/video/BV1xx411c7mD?p=1")
        );
    }
}
