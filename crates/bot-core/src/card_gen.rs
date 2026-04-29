use std::time::Duration;

use headless_chrome::protocol::cdp::Emulation;
use headless_chrome::{Browser, LaunchOptions};
use log::{error, info, warn};
use qrcode::render::svg;
use qrcode::QrCode;
use tokio::sync::{mpsc, oneshot};

use crate::error::BotError;
use crate::util::base64_encode;
struct RenderRequest {
    html: String,
    w: u32,
    h: u32,
    result_tx: oneshot::Sender<Result<String, BotError>>,
}

/// Queue-based render manager. A single background task owns the browser,
/// eliminating Mutex contention and deadlocks.
pub struct RenderManager {
    tx: mpsc::Sender<RenderRequest>,
}

/// Generate a QR code SVG data URI for the given text.
pub fn generate_qr_data_uri(text: &str) -> String {
    let code = match QrCode::new(text.as_bytes()) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let svg = code
        .render()
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();
    format!(
        "data:image/svg+xml;base64,{}",
        base64_encode(svg.as_bytes())
    )
}

impl RenderManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<RenderRequest>(64);
        tokio::spawn(async move {
            render_loop(rx).await;
        });
        Self { tx }
    }

    pub async fn render(&self, html: String, w: u32, h: u32) -> Result<String, BotError> {
        let (result_tx, result_rx) = oneshot::channel();
        self.tx
            .send(RenderRequest {
                html,
                w,
                h,
                result_tx,
            })
            .await
            .map_err(|e| BotError::Render(format!("Render queue disconnected: {}", e)))?;
        result_rx
            .await
            .map_err(|e| BotError::Render(format!("Render task cancelled: {}", e)))?
    }
}

async fn render_loop(mut rx: mpsc::Receiver<RenderRequest>) {
    let mut browser: Option<Browser> = None;
    let mut failures: u32 = 0;

    while let Some(req) = rx.recv().await {
        if browser.is_none() {
            match Browser::new(launch_options()) {
                Ok(b) => {
                    info!("RenderManager: browser launched");
                    browser = Some(b);
                    failures = 0;
                }
                Err(e) => {
                    error!("RenderManager: failed to launch browser: {}", e);
                    let _ = req.result_tx.send(Err(BotError::Render(e.to_string())));
                    continue;
                }
            }
        }

        let browser_ref = browser.as_ref().unwrap();
        let started = std::time::Instant::now();
        match render_in_browser(browser_ref, &req.html, req.w, req.h) {
            Ok(b64) => {
                failures = 0;
                let elapsed = started.elapsed();
                if elapsed > Duration::from_secs(10) {
                    warn!(
                        "RenderManager: slow render took {:.1}s",
                        elapsed.as_secs_f64()
                    );
                }
                let _ = req.result_tx.send(Ok(b64));
            }
            Err(e) => {
                failures += 1;
                error!("RenderManager: render failed (failures={failures}): {e}");
                let _ = req.result_tx.send(Err(BotError::Render(e)));

                if failures >= 3 {
                    warn!("RenderManager: recycling browser after {failures} consecutive failures");
                    browser = None;
                    failures = 0;
                }
            }
        }
    }

    if let Some(b) = browser.take() {
        drop(b);
        info!("RenderManager: browser closed");
    }
    info!("RenderManager: render loop shutting down");
}

fn launch_options() -> LaunchOptions<'static> {
    LaunchOptions {
        headless: true,
        sandbox: false,
        enable_gpu: false,
        idle_browser_timeout: std::time::Duration::from_secs(3600 * 24),
        ..LaunchOptions::default()
    }
}

fn render_in_browser(browser: &Browser, html: &str, w: u32, h: u32) -> Result<String, String> {
    let tab = browser
        .new_tab()
        .map_err(|e| format!("Failed to create tab: {}", e))?;

    tab.call_method(Emulation::SetDeviceMetricsOverride {
        width: w,
        height: h,
        device_scale_factor: 2.0,
        mobile: false,
        scale: None,
        screen_width: None,
        screen_height: None,
        position_x: None,
        position_y: None,
        dont_set_visible_size: None,
        screen_orientation: None,
        viewport: None,
        display_feature: None,
        device_posture: None,
    })
    .map_err(|e| format!("Failed to set device metrics: {}", e))?;

    let encoded = base64_encode(html.as_bytes());
    let data_url = format!("data:text/html;charset=utf-8;base64,{}", encoded);

    tab.navigate_to(&data_url)
        .map_err(|e| format!("Failed to navigate: {}", e))?;

    tab.wait_until_navigated()
        .map_err(|e| format!("Failed to wait for navigation: {}", e))?;

    let _ = tab
        .wait_for_element("body")
        .map_err(|e| format!("Failed to find body element: {}", e))?;

    let png = tab
        .capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            true,
        )
        .map_err(|e| format!("Failed to capture screenshot: {}", e))?;

    tab.close(true)
        .map_err(|e| format!("Failed to close tab: {}", e))?;

    Ok(base64_encode(&png))
}
