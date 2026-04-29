use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::action::{ApiRequest, ApiResponse};
use crate::error::{NapError, Result};
use crate::event::{MessageEvent, NoticeEvent};
use crate::message::{ForwardNode, Message};
use crate::model::{FriendInfo, GroupInfo};

type EventCallback<T> = Arc<dyn Fn(T) + Send + Sync + 'static>;

struct ClientInner {
    /// Current connection's outgoing channel. None when no client connected.
    event_tx: RwLock<Option<mpsc::UnboundedSender<String>>>,
    pending: RwLock<HashMap<String, oneshot::Sender<ApiResponse>>>,
    on_message: RwLock<Option<EventCallback<MessageEvent>>>,
    on_notice: RwLock<Option<EventCallback<NoticeEvent>>>,
}

#[derive(Clone)]
pub struct NapClient {
    inner: Arc<ClientInner>,
}

impl NapClient {
    pub async fn bind(addr: &str, token: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| NapError::Ws(tokio_tungstenite::tungstenite::Error::Io(e)))?;
        info!("NapCat WS server listening on {}", addr);

        let client = NapClient {
            inner: Arc::new(ClientInner {
                event_tx: RwLock::new(None),
                pending: RwLock::new(HashMap::new()),
                on_message: RwLock::new(None),
                on_notice: RwLock::new(None),
            }),
        };

        let inner = Arc::clone(&client.inner);
        let expected_auth = format!("Bearer {}", token);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        info!("NapCat connecting from {}", peer_addr);

                        let auth = expected_auth.clone();
                        #[allow(clippy::result_large_err)]
                        match accept_hdr_async(stream, |req: &Request, resp: Response| {
                            let got = req
                                .headers()
                                .get("Authorization")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("");
                            if got == auth || auth == "Bearer " {
                                Ok(resp)
                            } else {
                                warn!(
                                    "WS auth rejected: {:?}",
                                    req.headers()
                                        .get("Authorization")
                                        .and_then(|v| v.to_str().ok())
                                );
                                Err(ErrorResponse::new(Some("Unauthorized".into())))
                            }
                        })
                        .await
                        {
                            Ok(ws_stream) => {
                                let (write, read) = ws_stream.split();
                                let (event_tx, event_rx) = mpsc::unbounded_channel();
                                *inner.event_tx.write().await = Some(event_tx);

                                let inner_for_read = Arc::clone(&inner);
                                let (response_tx, response_rx) =
                                    mpsc::unbounded_channel::<WsMessage>();

                                // Read task: parse incoming messages
                                let read_handle = tokio::spawn(async move {
                                    let mut read = read;
                                    while let Some(msg) = read.next().await {
                                        match msg {
                                            Ok(WsMessage::Text(text)) => {
                                                let text = text.to_string();
                                                debug!("WS recv: {}", truncate_str(&text, 300));
                                                if let Ok(resp) =
                                                    serde_json::from_str::<ApiResponse>(&text)
                                                {
                                                    let mut map =
                                                        inner_for_read.pending.write().await;
                                                    if let Some(tx) = map.remove(&resp.echo) {
                                                        let _ = tx.send(resp);
                                                    }
                                                } else if let Ok(event) =
                                                    serde_json::from_str::<MessageEvent>(&text)
                                                {
                                                    debug!("MessageEvent: {:?}", event);
                                                    if event.post_type == "message" {
                                                        if let Some(ref cb) =
                                                            *inner_for_read.on_message.read().await
                                                        {
                                                            cb(event);
                                                        }
                                                    }
                                                } else if let Ok(event) =
                                                    serde_json::from_str::<NoticeEvent>(&text)
                                                {
                                                    debug!("NoticeEvent: {:?}", event);
                                                    if event.post_type == "notice" {
                                                        if let Some(ref cb) =
                                                            *inner_for_read.on_notice.read().await
                                                        {
                                                            cb(event);
                                                        }
                                                    }
                                                } else if let Ok(meta) =
                                                    serde_json::from_str::<crate::event::MetaEvent>(
                                                        &text,
                                                    )
                                                {
                                                    info!(
                                                        "Meta: {} {}",
                                                        meta.meta_event_type, meta.sub_type
                                                    );
                                                } else {
                                                    warn!(
                                                        "Unrecognized: {}",
                                                        &text[..text.len().min(200)]
                                                    );
                                                }
                                            }
                                            Ok(WsMessage::Ping(data)) => {
                                                let _ = response_tx.send(WsMessage::Pong(data));
                                            }
                                            Ok(WsMessage::Close(_)) => {
                                                info!("NapCat disconnected");
                                                break;
                                            }
                                            Ok(_) => {}
                                            Err(e) => {
                                                error!("WS read error: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                });

                                // Write task: send messages from response_rx and event_rx
                                let mut write = write;
                                let read_abort = read_handle.abort_handle();
                                let write_handle = tokio::spawn(async move {
                                    let mut response_rx = response_rx;
                                    let mut event_rx = event_rx;
                                    loop {
                                        tokio::select! {
                                            Some(msg) = response_rx.recv() => {
                                                if let Err(e) = write.send(msg).await {
                                                    error!("WS write error: {}", e);
                                                    break;
                                                }
                                            }
                                            Some(text) = event_rx.recv() => {
                                                if let Err(e) = write.send(WsMessage::Text(text)).await {
                                                    error!("WS write error: {}", e);
                                                    break;
                                                }
                                            }
                                            else => break,
                                        }
                                    }
                                });

                                let _ = read_handle.await;
                                read_abort.abort();
                                write_handle.abort();

                                // Clear state for next connection
                                *inner.event_tx.write().await = None;
                                inner.pending.write().await.clear();
                                info!("NapCat connection cleaned up");
                            }
                            Err(e) => error!("WS handshake failed: {}", e),
                        }
                    }
                    Err(e) => error!("Accept failed: {}", e),
                }
            }
        });

        Ok(client)
    }

    pub fn on_message(&self, f: impl Fn(MessageEvent) + Send + Sync + 'static) {
        match self.inner.on_message.try_write() {
            Ok(mut guard) => *guard = Some(Arc::new(f)),
            Err(_) => warn!("on_message callback race"),
        }
    }

    pub async fn is_connected(&self) -> bool {
        self.inner.event_tx.read().await.is_some()
    }

    pub fn on_notice(&self, f: impl Fn(NoticeEvent) + Send + Sync + 'static) {
        match self.inner.on_notice.try_write() {
            Ok(mut guard) => *guard = Some(Arc::new(f)),
            Err(_) => warn!("on_notice callback race"),
        }
    }

    pub async fn send_to_group(&self, group_id: i64, msg: Message) -> Result<i64> {
        let req = ApiRequest::send_group_msg(group_id, &msg);
        let resp = self.send_request(req).await?;
        resp.data["message_id"].as_i64().ok_or(NapError::Api {
            retcode: resp.retcode,
            status: resp.status,
        })
    }

    pub async fn send_to_user(&self, user_id: i64, msg: Message) -> Result<i64> {
        let req = ApiRequest::send_private_msg(user_id, &msg);
        let resp = self.send_request(req).await?;
        resp.data["message_id"].as_i64().ok_or(NapError::Api {
            retcode: resp.retcode,
            status: resp.status,
        })
    }

    pub async fn get_friend_list(&self) -> Result<Vec<FriendInfo>> {
        let req = ApiRequest::get_friend_list();
        let resp = self.send_request(req).await?;
        serde_json::from_value(resp.data).map_err(NapError::Json)
    }

    pub async fn get_group_list(&self) -> Result<Vec<GroupInfo>> {
        let req = ApiRequest::get_group_list();
        let resp = self.send_request(req).await?;
        serde_json::from_value(resp.data).map_err(NapError::Json)
    }

    pub async fn send_group_forward_msg(
        &self,
        group_id: i64,
        nodes: &[ForwardNode],
    ) -> Result<i64> {
        let req = ApiRequest::send_group_forward_msg(group_id, nodes);
        let resp = self.send_request(req).await?;
        resp.data["message_id"].as_i64().ok_or(NapError::Api {
            retcode: resp.retcode,
            status: resp.status,
        })
    }

    pub async fn send_private_forward_msg(
        &self,
        user_id: i64,
        nodes: &[ForwardNode],
    ) -> Result<i64> {
        let req = ApiRequest::send_private_forward_msg(user_id, nodes);
        let resp = self.send_request(req).await?;
        resp.data["message_id"].as_i64().ok_or(NapError::Api {
            retcode: resp.retcode,
            status: resp.status,
        })
    }

    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse> {
        let echo = req.echo.clone();
        let (tx, rx) = oneshot::channel();

        self.inner.pending.write().await.insert(echo.clone(), tx);

        let payload = serde_json::to_string(&req)?;

        let guard = self.inner.event_tx.read().await;
        let tx = guard.as_ref().ok_or(NapError::NoClient)?;
        tx.send(payload).map_err(|_| NapError::NoClient)?;
        drop(guard);

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(resp)) if resp.retcode == 0 => Ok(resp),
            Ok(Ok(resp)) => Err(NapError::Api {
                retcode: resp.retcode,
                status: resp.status,
            }),
            Ok(Err(_)) => Err(NapError::ConnectionClosed),
            Err(_) => {
                self.inner.pending.write().await.remove(&echo);
                Err(NapError::Timeout { echo })
            }
        }
    }
}

fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_ascii() {
        assert_eq!(truncate_str("hello", 3), "hel");
    }

    #[test]
    fn truncate_chinese() {
        assert_eq!(truncate_str("你好世界", 7), "你好");
        assert_eq!(truncate_str("你好世界", 3), "你");
        assert_eq!(truncate_str("你好世界", 1), "");
    }

    #[test]
    fn truncate_noop() {
        assert_eq!(truncate_str("short", 100), "short");
    }
}
