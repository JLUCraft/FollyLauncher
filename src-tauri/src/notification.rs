use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use tokio::sync::Mutex;

pub struct NotificationService {
    last_seen: Arc<Mutex<Option<String>>>,
}

impl NotificationService {
    pub fn new() -> Self {
        Self {
            last_seen: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(self: Arc<Self>, handle: tauri::AppHandle) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                // Try to access the managed state
                let state = handle.state::<Arc<Mutex<crate::AppState>>>();

                // Get latest messages
                let messages: Vec<crate::network::ClusterMessage> = {
                    let state = state.lock().await;
                    state.network.get_messages().await
                };

                if let Some(latest) = messages.last() {
                    let new_id = format!("{}-{}", latest.topic, latest.received_at);
                    let mut last = self.last_seen.lock().await;

                    if last.as_deref() != Some(&new_id) {
                        *last = Some(new_id.clone());

                        let title = match latest.topic.as_str() {
                            "mc.events.cluster" => "集群事件",
                            "mc.events.governance" => "治理事件",
                            _ => "平台通知",
                        };

                        let body = match latest.payload.get("message") {
                            Some(serde_json::Value::String(msg)) => msg.clone(),
                            _ => String::from("收到新的平台消息"),
                        };

                        // Send native notification
                        use tauri_plugin_notification::NotificationExt;
                        let notification = handle
                            .notification()
                            .builder()
                            .title(title)
                            .body(&body)
                            .show();

                        if let Err(e) = notification {
                            tracing::warn!(error = %e, "failed to send notification");
                        }
                    }
                }
            }
        });
    }
}
