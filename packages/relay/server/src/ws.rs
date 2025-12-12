//! Minimal WebSocket support for actix-web.
//!
//! This module replaces `actix-ws` to avoid compatibility issues with `actix-web` 4.12+.
//! It provides only the functionality needed by this crate.

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use actix_codec::{Decoder, Encoder};
use actix_http::{
    Payload,
    body::BodyStream,
    ws::{Codec, Frame, HandshakeError},
};
use actix_web::{HttpRequest, HttpResponse, web};
use bytes::{Bytes, BytesMut};
use bytestring::ByteString;
use futures_core::Stream;
use tokio::sync::mpsc;

// Re-export types from actix-http that consumers need
pub use actix_http::ws::{Message, ProtocolError};

/// Error returned when the WebSocket session is closed.
#[derive(Debug)]
pub struct Closed;

/// A WebSocket session for sending messages to the client.
pub struct Session {
    tx: mpsc::Sender<Message>,
}

impl Session {
    const fn new(tx: mpsc::Sender<Message>) -> Self {
        Self { tx }
    }

    /// Send a text message to the client.
    ///
    /// # Errors
    /// Returns `Closed` if the session has been closed.
    pub async fn text(&self, msg: impl Into<ByteString>) -> Result<(), Closed> {
        self.tx
            .send(Message::Text(msg.into()))
            .await
            .map_err(|_| Closed)
    }

    /// Send a pong message to the client.
    ///
    /// # Errors
    /// Returns `Closed` if the session has been closed.
    pub async fn pong(&self, msg: &[u8]) -> Result<(), Closed> {
        self.tx
            .send(Message::Pong(Bytes::copy_from_slice(msg)))
            .await
            .map_err(|_| Closed)
    }
}

/// Stream of WebSocket messages from the client.
pub struct MessageStream {
    payload: Payload,
    buf: BytesMut,
    codec: Codec,
    closing: bool,
}

impl MessageStream {
    fn new(payload: Payload) -> Self {
        Self {
            payload,
            buf: BytesMut::new(),
            codec: Codec::new(),
            closing: false,
        }
    }
}

impl Stream for MessageStream {
    type Item = Result<Message, ProtocolError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if !this.closing {
            // Read bytes from payload until pending or done
            loop {
                match Pin::new(&mut this.payload).poll_next(cx) {
                    Poll::Ready(Some(Ok(bytes))) => {
                        this.buf.extend_from_slice(&bytes);
                    }
                    Poll::Ready(Some(Err(err))) => {
                        return Poll::Ready(Some(Err(ProtocolError::Io(io::Error::other(err)))));
                    }
                    Poll::Ready(None) => {
                        this.closing = true;
                        break;
                    }
                    Poll::Pending => break,
                }
            }
        }

        // Try to decode a frame from the buffer
        match this.codec.decode(&mut this.buf) {
            Ok(Some(frame)) => {
                let message =
                    match frame {
                        Frame::Text(bytes) => ByteString::try_from(bytes)
                            .map(Message::Text)
                            .map_err(|err| {
                                ProtocolError::Io(io::Error::new(io::ErrorKind::InvalidData, err))
                            })?,
                        Frame::Binary(bytes) => Message::Binary(bytes),
                        Frame::Ping(bytes) => Message::Ping(bytes),
                        Frame::Pong(bytes) => Message::Pong(bytes),
                        Frame::Close(reason) => Message::Close(reason),
                        Frame::Continuation(item) => Message::Continuation(item),
                    };
                Poll::Ready(Some(Ok(message)))
            }
            Ok(None) => {
                if this.closing {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
            Err(err) => Poll::Ready(Some(Err(err))),
        }
    }
}

/// Response body that streams WebSocket messages to the client.
struct StreamingBody {
    rx: mpsc::Receiver<Message>,
    buf: BytesMut,
    codec: Codec,
    closing: bool,
}

impl StreamingBody {
    fn new(rx: mpsc::Receiver<Message>) -> Self {
        Self {
            rx,
            buf: BytesMut::new(),
            codec: Codec::new(),
            closing: false,
        }
    }
}

impl Stream for StreamingBody {
    type Item = Result<Bytes, actix_web::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.closing {
            return Poll::Ready(None);
        }

        // Receive messages from the session and encode them
        loop {
            match Pin::new(&mut this.rx).poll_recv(cx) {
                Poll::Ready(Some(msg)) => {
                    if let Err(err) = this.codec.encode(msg, &mut this.buf) {
                        return Poll::Ready(Some(Err(actix_web::error::ErrorInternalServerError(
                            err,
                        ))));
                    }
                }
                Poll::Ready(None) => {
                    this.closing = true;
                    break;
                }
                Poll::Pending => break,
            }
        }

        if !this.buf.is_empty() {
            let bytes = this.buf.split().freeze();
            return Poll::Ready(Some(Ok(bytes)));
        }

        if this.closing {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

/// Perform WebSocket handshake and return session/stream handles.
///
/// # Errors
/// Returns an error if the handshake fails.
pub fn handle(
    req: &HttpRequest,
    body: web::Payload,
) -> Result<(HttpResponse, Session, MessageStream), actix_web::Error> {
    let mut response = actix_http::ws::handshake(req.head()).map_err(|err| match err {
        HandshakeError::NoConnectionUpgrade => {
            actix_web::error::ErrorBadRequest("No Connection upgrade")
        }
        HandshakeError::NoWebsocketUpgrade => {
            actix_web::error::ErrorBadRequest("No WebSocket upgrade")
        }
        HandshakeError::NoVersionHeader => {
            actix_web::error::ErrorBadRequest("No WebSocket version header")
        }
        HandshakeError::UnsupportedVersion => {
            actix_web::error::ErrorBadRequest("Unsupported WebSocket version")
        }
        HandshakeError::BadWebsocketKey => actix_web::error::ErrorBadRequest("Bad WebSocket key"),
        HandshakeError::GetMethodRequired => {
            actix_web::error::ErrorMethodNotAllowed("GET method required")
        }
    })?;

    let (tx, rx) = mpsc::channel(32);

    let body_stream = BodyStream::new(StreamingBody::new(rx));
    let response = response.body(body_stream).map_into_boxed_body();

    Ok((
        HttpResponse::from(response),
        Session::new(tx),
        MessageStream::new(body.into_inner()),
    ))
}
