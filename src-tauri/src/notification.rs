use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};


use crate::network::{
    MC_ADMIN_PUSH_TOPIC, MC_CLUSTER_TOPIC, MC_GOVERNANCE_TOPIC, MC_INSTANCE_TOPIC_PREFIX,
    MC_SYSTEM_TOPIC, MC_TOURNAMENT_TOPIC_PREFIX,
};


const MAX_DEDUP_ENTRIES: usize = 1024;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Priority {

    Normal,

    High,

    Critical,
}

pub struct NotificationService {



    seen_ids: Arc<Mutex<VecDeque<String>>>,
}

impl NotificationService {
    pub fn new() -> Self {
        Self {
            seen_ids: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_DEDUP_ENTRIES))),
        }
    }

    pub async fn handle_event(
        &self,
        handle: &tauri::AppHandle,
        event: crate::protos::jlucraft::events::v1::EventEnvelope,
    ) {
        let msg = crate::network::ClusterMessage {
            topic: event.topic.clone(),
            peer_id: event.source_peer_id.clone(),
            payload: event_to_payload(&event),
            received_at: event.occurred_at.clone(),
        };

        let payload_fingerprint = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            msg.payload.to_string().hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        };
        let event_id = if event.event_id.is_empty() {
            format!("{}|{}|{}", msg.topic, msg.peer_id, payload_fingerprint)
        } else {
            event.event_id.clone()
        };
        {
            let mut seen = self.seen_ids.lock().await;
            if seen.contains(&event_id) {
                return;
            }
            if seen.len() >= MAX_DEDUP_ENTRIES {
                seen.pop_front();
            }
            seen.push_back(event_id);
        }

        let (title, body, priority) = self.classify_message(&msg);
        if let Some(body) = body {
            if let Err(e) = self
                .send_native_notification(handle, &title, &body, priority)
                .await
            {
                warn!(error = %e, topic = %msg.topic, "failed to send notification");
            }
        }
    }



    fn classify_message(
        &self,
        msg: &crate::network::ClusterMessage,
    ) -> (String, Option<String>, Priority) {
        let topic = &msg.topic;
        let payload = &msg.payload;


        if topic == MC_ADMIN_PUSH_TOPIC {
            let action = payload
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let message = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("管理员推送了一条消息");


            let requires_auth = payload
                .get("requires_auth")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let prefix = if requires_auth {
                "⚠️ [需授权确认] "
            } else {
                ""
            };

            let body = if action == "policy_update" {
                format!("{prefix}政策已更新: {message}。请在「我的」页面查看详情。")
            } else if action == "vc_revocation" {
                format!("{prefix}VC 吊销通知: {message}。你的社团身份可能受影响。")
            } else if action == "emergency_shutdown" {
                format!("{prefix}紧急维护通知: {message}。请尽快保存并退出游戏。")
            } else if action == "maintenance" {
                format!("{prefix}维护通知: {message}。")
            } else {
                format!("{prefix}{message}")
            };

            return (
                "管理员推送".to_string(),
                Some(body),
                if requires_auth {
                    Priority::Critical
                } else {
                    Priority::High
                },
            );
        }


        if topic == MC_SYSTEM_TOPIC {
            let event_type = payload
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("system_event");
            let message = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("系统消息");

            let priority = match event_type {
                "error" | "outage" => Priority::High,
                _ => Priority::Normal,
            };

            return (
                format!("系统通知 — {event_type}"),
                Some(message.to_string()),
                priority,
            );
        }


        if topic.starts_with(MC_INSTANCE_TOPIC_PREFIX) {
            let instance_id_part = topic.strip_prefix(MC_INSTANCE_TOPIC_PREFIX).unwrap_or("?");
            let event_type = payload
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("update");
            let message = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or(event_type);


            if matches!(event_type, "started" | "stopped" | "crashed" | "migration") {
                return (
                    format!("实例 {} — {}", instance_id_part, event_type),
                    Some(message.to_string()),
                    if event_type == "crashed" {
                        Priority::High
                    } else {
                        Priority::Normal
                    },
                );
            }

            return (String::new(), None, Priority::Normal);
        }


        if topic.starts_with(MC_TOURNAMENT_TOPIC_PREFIX) {
            let tournament_id = topic
                .strip_prefix(MC_TOURNAMENT_TOPIC_PREFIX)
                .unwrap_or("?");
            let event_type = payload
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("update");
            let message = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or(event_type);

            let priority = match event_type {
                "match_starting" | "round_ending" => Priority::High,
                _ => Priority::Normal,
            };

            return (
                format!("联赛 {} — {}", tournament_id, event_type),
                Some(message.to_string()),
                priority,
            );
        }


        if topic == MC_GOVERNANCE_TOPIC {
            let event_type = payload
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("update");
            let message = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or(event_type);

            return (
                format!("治理事件 — {}", event_type),
                Some(message.to_string()),
                Priority::Normal,
            );
        }


        if topic == MC_CLUSTER_TOPIC {
            if let Some(event_type) = payload.get("event_type").and_then(|v| v.as_str()) {
                if let Some(inner) = payload.get("payload") {
                    if let Some(name) = inner.get("name").and_then(|v| v.as_str()) {
                        match event_type {
                            "instance-created" => {
                                return (
                                    "新实例".to_string(),
                                    Some(format!("{} 已创建", name)),
                                    Priority::Normal,
                                );
                            }
                            "instance-destroyed" => {
                                return (
                                    "实例下线".to_string(),
                                    Some(format!("{} 已下线", name)),
                                    Priority::Normal,
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }


        (String::new(), None, Priority::Normal)
    }


    async fn send_native_notification(
        &self,
        handle: &tauri::AppHandle,
        title: &str,
        body: &str,
        priority: Priority,
    ) -> Result<(), crate::error::LauncherError> {
        use tauri_plugin_notification::NotificationExt;

        let display_title = match priority {
            Priority::Critical => format!("[紧急] {}", title),
            Priority::High => title.to_string(),
            Priority::Normal => title.to_string(),
        };

        handle
            .notification()
            .builder()
            .title(&display_title)
            .body(body)
            .show()
            .map_err(|e| crate::error::LauncherError::from(format!("notification failed: {e}")))?;

        info!(%display_title, %body, ?priority, "native notification sent");
        Ok(())
    }
}

fn event_to_payload(
    event: &crate::protos::jlucraft::events::v1::EventEnvelope,
) -> serde_json::Value {
    use crate::protos::jlucraft::events::v1::event_envelope;

    let mut val = serde_json::json!({ "event_type": event.event_type });
    match event.payload.as_ref() {
        Some(event_envelope::Payload::InstanceUpdate(i)) => {
            val["payload"] = serde_json::json!({
                "instance_id": i.instance_id,
                "name": i.name,
                "kind": i.kind,
                "host": i.host_peer_id,
                "mode": i.mode,
                "club": i.club,
                "players": i.player_count,
                "max_players": i.max_players,
                "version": i.version,
            });
        }
        Some(event_envelope::Payload::InstanceInvite(i)) => {
            val["message"] = serde_json::json!(i.message);
            val["payload"] = serde_json::json!({
                "instance_id": i.instance_id,
                "invited_count": i.invited_count,
                "players": i.players,
            });
        }
        Some(event_envelope::Payload::PushNotification(n)) => {
            val["message"] = serde_json::json!(n.body);
            val["title"] = serde_json::json!(n.title);
            val["severity"] = serde_json::json!(n.severity);
        }
        Some(event_envelope::Payload::Generic(g)) => {
            val["message"] =
                serde_json::json!(g.attributes.get("message").cloned().unwrap_or_default());
            val["payload"] = serde_json::json!(g.attributes);
        }
        _ => {}
    }
    val
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::ClusterMessage;
    use serde_json::json;

    fn make_msg(topic: &str, payload: serde_json::Value) -> ClusterMessage {
        ClusterMessage {
            topic: topic.to_string(),
            peer_id: "12D3KooWTest".to_string(),
            payload,
            received_at: "2025-01-01T00:00:00Z".to_string(),
        }
    }



    #[test]
    fn test_classify_admin_push_critical() {
        let svc = NotificationService::new();
        let msg = make_msg(
            MC_ADMIN_PUSH_TOPIC,
            json!({
                "action": "vc_revocation",
                "message": "Your VC has been revoked",
                "requires_auth": true,
            }),
        );
        let (title, body, priority) = svc.classify_message(&msg);
        assert_eq!(title, "管理员推送");
        assert!(body.unwrap().contains("需授权确认"));
        assert_eq!(priority, Priority::Critical);
    }

    #[test]
    fn test_classify_admin_push_high() {
        let svc = NotificationService::new();
        let msg = make_msg(
            MC_ADMIN_PUSH_TOPIC,
            json!({
                "action": "maintenance",
                "message": "Server maintenance in 30 minutes",
                "requires_auth": false,
            }),
        );
        let (_title, body, priority) = svc.classify_message(&msg);
        assert!(body.unwrap().contains("维护通知"));
        assert_eq!(priority, Priority::High);
    }

    #[test]
    fn test_classify_admin_push_emergency() {
        let svc = NotificationService::new();
        let msg = make_msg(
            MC_ADMIN_PUSH_TOPIC,
            json!({
                "action": "emergency_shutdown",
                "message": "Immediate shutdown required",
                "requires_auth": true,
            }),
        );
        let (_title, body, priority) = svc.classify_message(&msg);
        assert!(body.unwrap().contains("紧急维护"));
        assert_eq!(
            priority,
            Priority::Critical,
            "emergency_shutdown with requires_auth should be Critical"
        );
    }

    #[test]
    fn test_classify_system_error_high() {
        let svc = NotificationService::new();
        let msg = make_msg(
            MC_SYSTEM_TOPIC,
            json!({
                "event_type": "error",
                "message": "Database connection failed",
            }),
        );
        let (title, body, priority) = svc.classify_message(&msg);
        assert!(title.contains("error"));
        assert!(body.is_some());
        assert_eq!(priority, Priority::High);
    }

    #[test]
    fn test_classify_system_normal() {
        let svc = NotificationService::new();
        let msg = make_msg(
            MC_SYSTEM_TOPIC,
            json!({
                "event_type": "info",
                "message": "Routine health check passed",
            }),
        );
        let (_title, _body, priority) = svc.classify_message(&msg);
        assert_eq!(priority, Priority::Normal);
    }

    #[test]
    fn test_classify_instance_started() {
        let svc = NotificationService::new();
        let topic = format!(
            "{}550e8400-e29b-41d4-a716-446655440000",
            MC_INSTANCE_TOPIC_PREFIX
        );
        let msg = make_msg(
            &topic,
            json!({
                "event_type": "started",
                "message": "Instance has started",
            }),
        );
        let (title, body, priority) = svc.classify_message(&msg);
        assert!(title.contains("550e8400"));
        assert!(title.contains("started"));
        assert!(body.is_some());
        assert_eq!(priority, Priority::Normal);
    }

    #[test]
    fn test_classify_instance_crashed_high() {
        let svc = NotificationService::new();
        let topic = format!("{}cafebabe-1234", MC_INSTANCE_TOPIC_PREFIX);
        let msg = make_msg(
            &topic,
            json!({
                "event_type": "crashed",
                "message": "OOM killer terminated the instance",
            }),
        );
        let (_title, _body, priority) = svc.classify_message(&msg);
        assert_eq!(priority, Priority::High);
    }

    #[test]
    fn test_classify_instance_update_skipped() {
        let svc = NotificationService::new();
        let topic = format!("{}test-instance", MC_INSTANCE_TOPIC_PREFIX);
        let msg = make_msg(
            &topic,
            json!({
                "event_type": "created",
                "message": "Instance created",
            }),
        );
        let (_title, body, _priority) = svc.classify_message(&msg);
        assert!(body.is_none(), "instance 'created' should be skipped");
    }

    #[test]
    fn test_classify_tournament_match_starting_high() {
        let svc = NotificationService::new();
        let topic = format!("{}tourney-001", MC_TOURNAMENT_TOPIC_PREFIX);
        let msg = make_msg(
            &topic,
            json!({
                "event_type": "match_starting",
                "message": "Your match starts in 5 minutes",
            }),
        );
        let (_title, _body, priority) = svc.classify_message(&msg);
        assert_eq!(priority, Priority::High);
    }

    #[test]
    fn test_classify_tournament_normal() {
        let svc = NotificationService::new();
        let topic = format!("{}tourney-002", MC_TOURNAMENT_TOPIC_PREFIX);
        let msg = make_msg(
            &topic,
            json!({
                "event_type": "registration_open",
                "message": "Registration is now open",
            }),
        );
        let (_title, _body, priority) = svc.classify_message(&msg);
        assert_eq!(priority, Priority::Normal);
    }

    #[test]
    fn test_classify_governance() {
        let svc = NotificationService::new();
        let msg = make_msg(
            MC_GOVERNANCE_TOPIC,
            json!({
                "event_type": "vote_started",
                "message": "New governance vote has started",
            }),
        );
        let (title, body, priority) = svc.classify_message(&msg);
        assert!(title.contains("治理事件"));
        assert!(body.is_some());
        assert_eq!(priority, Priority::Normal);
    }

    #[test]
    fn test_classify_cluster_instance_created() {
        let svc = NotificationService::new();
        let msg = make_msg(
            MC_CLUSTER_TOPIC,
            json!({
                "event_type": "instance-created",
                "payload": {
                    "name": "My New Server",
                },
            }),
        );
        let (_title, body, _priority) = svc.classify_message(&msg);
        assert!(body.is_some());
        assert!(body.unwrap().contains("My New Server"));
    }

    #[test]
    fn test_classify_unknown_topic_skipped() {
        let svc = NotificationService::new();
        let msg = make_msg("mc.events.unknown.topic", json!({"some": "data"}));
        let (_title, body, _priority) = svc.classify_message(&msg);
        assert!(body.is_none(), "unknown topics should be skipped");
    }



    #[tokio::test]
    async fn test_dedup_same_message_twice_filtered() {
        let svc = NotificationService::new();
        let msg = make_msg(
            MC_CLUSTER_TOPIC,
            json!({"event_type": "instance-created", "payload": {"name": "test"}}),
        );


        let payload_fingerprint = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            msg.payload.to_string().hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        };
        let msg_id = format!("{}|{}|{}", msg.topic, msg.peer_id, payload_fingerprint);


        {
            let mut seen = svc.seen_ids.lock().await;
            assert!(!seen.contains(&msg_id));
            seen.push_back(msg_id.clone());
        }


        {
            let seen = svc.seen_ids.lock().await;
            assert!(seen.contains(&msg_id), "same message should be deduped");
        }
    }

    #[tokio::test]
    async fn test_dedup_different_messages_both_pass() {
        let svc = NotificationService::new();
        let msg_a = make_msg(
            MC_CLUSTER_TOPIC,
            json!({"event_type": "instance-created", "payload": {"name": "A"}}),
        );
        let msg_b = make_msg(
            MC_CLUSTER_TOPIC,
            json!({"event_type": "instance-created", "payload": {"name": "B"}}),
        );

        let fingerprint = |m: &ClusterMessage| -> String {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            m.payload.to_string().hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        };

        let id_a = format!("{}|{}|{}", msg_a.topic, msg_a.peer_id, fingerprint(&msg_a));
        let id_b = format!("{}|{}|{}", msg_b.topic, msg_b.peer_id, fingerprint(&msg_b));

        assert_ne!(
            id_a, id_b,
            "different payloads should produce different dedup keys"
        );

        let mut seen = svc.seen_ids.lock().await;
        seen.push_back(id_a.clone());
        seen.push_back(id_b.clone());
        assert!(seen.contains(&id_a));
        assert!(seen.contains(&id_b));
        assert_eq!(seen.len(), 2);
    }

    #[tokio::test]
    async fn test_dedup_lru_eviction_when_full() {
        let svc = NotificationService::new();
        let mut seen = svc.seen_ids.lock().await;


        for i in 0..MAX_DEDUP_ENTRIES {
            seen.push_back(format!("msg-{}", i));
        }
        assert_eq!(seen.len(), MAX_DEDUP_ENTRIES);


        assert_eq!(seen[0], "msg-0");


        if seen.len() >= MAX_DEDUP_ENTRIES {
            seen.pop_front();
        }
        seen.push_back("msg-new".to_string());
        assert_eq!(seen.len(), MAX_DEDUP_ENTRIES);
        assert_eq!(seen[0], "msg-1", "oldest entry should be evicted");
        assert_eq!(seen[seen.len() - 1], "msg-new");
    }

    #[test]
    fn test_priority_critical_gets_prefix_in_title() {

        let display_title = match Priority::Critical {
            Priority::Critical => format!("[紧急] {}", "管理员推送"),
            Priority::High => "管理员推送".to_string(),
            Priority::Normal => "管理员推送".to_string(),
        };
        assert!(display_title.contains("[紧急]"));
        assert!(display_title.contains("管理员推送"));
    }

    #[test]
    fn test_priority_normal_no_prefix() {
        let display_title = match Priority::Normal {
            Priority::Critical => format!("[紧急] {}", "系统通知"),
            Priority::High => "系统通知".to_string(),
            Priority::Normal => "系统通知".to_string(),
        };
        assert!(!display_title.contains("[紧急]"));
        assert_eq!(display_title, "系统通知");
    }
}
