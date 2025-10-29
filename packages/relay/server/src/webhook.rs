use actix_web::{HttpRequest, HttpResponse, web};
use chadreview_relay_models::{PrKey, RelayMessage, ServerMessage, WebhookEvent};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

#[allow(clippy::future_not_send)]
pub async fn handler(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> HttpResponse {
    let instance_id = path.into_inner();

    if let Err(e) = verify_github_signature(&req, &body, state.webhook_secret.as_deref()) {
        log::warn!("Invalid GitHub signature: {e}");
        return HttpResponse::Unauthorized().finish();
    }

    let event_type = req
        .headers()
        .get("X-GitHub-Event")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    drop(req);

    let event: WebhookEvent = match parse_webhook_event(event_type.as_deref(), &body) {
        Ok(event) => event,
        Err(e) => {
            log::error!("Failed to parse webhook: {e}");
            return HttpResponse::BadRequest().body(format!("Failed to parse webhook: {e}"));
        }
    };

    let pr_key = extract_pr_key(&event);

    let relay_msg = RelayMessage {
        instance_id: instance_id.clone(),
        pr_key: pr_key.clone(),
        event,
    };

    let instances = state.get_subscribed_instances(&pr_key).await;

    let server_msg = ServerMessage::Webhook(Box::new(relay_msg));
    let json = match serde_json::to_string(&server_msg) {
        Ok(json) => json,
        Err(e) => {
            log::error!("Failed to serialize message: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut sent_count = 0;
    for target_instance in instances {
        if let Some(senders) = state.connections.read().await.get(&target_instance) {
            for sender in senders {
                if sender.send(json.clone()).is_ok() {
                    sent_count += 1;
                }
            }
        }
    }

    log::info!(
        "Relayed webhook for PR {}/{} #{} to {} instance(s)",
        pr_key.owner,
        pr_key.repo,
        pr_key.number,
        sent_count
    );

    HttpResponse::Ok().finish()
}

fn verify_github_signature(
    req: &HttpRequest,
    body: &[u8],
    secret: Option<&str>,
) -> Result<(), &'static str> {
    if secret.is_none() {
        log::warn!("GITHUB_WEBHOOK_SECRET not set, skipping signature verification");
        return Ok(());
    }

    let signature_header = req
        .headers()
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing signature header")?;

    let expected_signature = signature_header
        .strip_prefix("sha256=")
        .ok_or("Invalid signature format")?;

    let mut mac =
        HmacSha256::new_from_slice(secret.unwrap().as_bytes()).map_err(|_| "Invalid secret")?;
    mac.update(body);

    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    let computed_signature = hex::encode(code_bytes);

    if computed_signature == expected_signature {
        Ok(())
    } else {
        Err("Signature mismatch")
    }
}

fn parse_webhook_event(
    event_type: Option<&str>,
    body: &[u8],
) -> Result<WebhookEvent, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_slice(body)?;

    match event_type {
        Some("issue_comment") => {
            let action = value["action"]
                .as_str()
                .and_then(|s| serde_json::from_str(&format!("\"{s}\"")).ok())
                .unwrap_or(chadreview_relay_models::CommentAction::Created);

            Ok(WebhookEvent::IssueComment {
                action,
                comment: serde_json::from_value(value["comment"].clone())?,
                issue: serde_json::from_value(value["issue"].clone())?,
                repository: serde_json::from_value(value["repository"].clone())?,
            })
        }
        Some("pull_request_review_comment") => {
            let action = value["action"]
                .as_str()
                .and_then(|s| serde_json::from_str(&format!("\"{s}\"")).ok())
                .unwrap_or(chadreview_relay_models::CommentAction::Created);

            Ok(WebhookEvent::PullRequestReviewComment {
                action,
                comment: Box::new(serde_json::from_value(value["comment"].clone())?),
                pull_request: serde_json::from_value(value["pull_request"].clone())?,
                repository: serde_json::from_value(value["repository"].clone())?,
            })
        }
        Some("pull_request") => {
            let action = value["action"]
                .as_str()
                .and_then(|s| serde_json::from_str(&format!("\"{s}\"")).ok())
                .unwrap_or(chadreview_relay_models::PrAction::Opened);

            Ok(WebhookEvent::PullRequest {
                action,
                pull_request: serde_json::from_value(value["pull_request"].clone())?,
                repository: serde_json::from_value(value["repository"].clone())?,
            })
        }
        _ => Err(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Unknown event type",
        ))),
    }
}

fn extract_pr_key(event: &WebhookEvent) -> PrKey {
    match event {
        WebhookEvent::IssueComment {
            issue, repository, ..
        } => PrKey {
            owner: repository.owner.login.clone(),
            repo: repository.name.clone(),
            number: issue.number,
        },
        WebhookEvent::PullRequestReviewComment {
            pull_request,
            repository,
            ..
        }
        | WebhookEvent::PullRequest {
            pull_request,
            repository,
            ..
        } => PrKey {
            owner: repository.owner.login.clone(),
            repo: repository.name.clone(),
            number: pull_request.number,
        },
    }
}
