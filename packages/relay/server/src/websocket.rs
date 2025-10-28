use actix_web::{HttpRequest, HttpResponse, web};
use actix_ws::Message;
use chadreview_relay_models::{ClientMessage, ServerMessage};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::state::AppState;

#[allow(clippy::future_not_send)]
pub async fn handler(
    req: HttpRequest,
    body: web::Payload,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let instance_id = path.into_inner();

    let (response, session, msg_stream) = actix_ws::handle(&req, body)?;

    let (tx, rx) = mpsc::unbounded_channel::<String>();

    state.add_connection(instance_id.clone(), tx).await;

    actix_web::rt::spawn(handle_websocket_connection(
        state.clone(),
        instance_id,
        session,
        msg_stream,
        rx,
    ));

    Ok(response)
}

#[allow(clippy::future_not_send)]
async fn handle_websocket_connection(
    state: web::Data<AppState>,
    instance_id: String,
    mut session: actix_ws::Session,
    mut msg_stream: actix_ws::MessageStream,
    mut rx: mpsc::UnboundedReceiver<String>,
) {
    log::info!("WebSocket connection established for instance: {instance_id}");

    loop {
        tokio::select! {
            Some(text) = rx.recv() => {
                if session.text(text).await.is_err() {
                    break;
                }
            }
            Some(Ok(msg)) = msg_stream.next() => {
                match msg {
                    Message::Text(text) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            handle_client_message(
                                &state,
                                &instance_id,
                                &mut session,
                                client_msg,
                            )
                            .await;
                        }
                    }
                    Message::Ping(bytes) => {
                        let _ = session.pong(&bytes).await;
                    }
                    Message::Close(_) => {
                        log::info!("Client closed connection: {instance_id}");
                        break;
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }

    state.remove_connection(&instance_id).await;
    log::info!("Connection closed for instance: {instance_id}");
}

async fn handle_client_message(
    state: &AppState,
    instance_id: &str,
    session: &mut actix_ws::Session,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::Subscribe(sub_msg) => {
            let pr_key = sub_msg.pr_key.clone();
            state
                .subscribe(instance_id.to_string(), pr_key.clone())
                .await;

            let response = ServerMessage::Subscribed {
                pr_key: pr_key.clone(),
            };
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = session.text(json).await;
            }

            log::info!(
                "Instance {} subscribed to PR {}/{} #{}",
                instance_id,
                pr_key.owner,
                pr_key.repo,
                pr_key.number
            );
        }
        ClientMessage::Unsubscribe(unsub_msg) => {
            let pr_key = unsub_msg.pr_key.clone();
            state.unsubscribe(instance_id, &pr_key).await;

            let response = ServerMessage::Unsubscribed {
                pr_key: pr_key.clone(),
            };
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = session.text(json).await;
            }

            log::info!(
                "Instance {} unsubscribed from PR {}/{} #{}",
                instance_id,
                pr_key.owner,
                pr_key.repo,
                pr_key.number
            );
        }
        ClientMessage::Ping => {
            let response = ServerMessage::Pong;
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = session.text(json).await;
            }
        }
    }
}
