use bytes::Bytes;

use futures_util::{Stream, StreamExt, stream};
use memchr::memmem::find;
use reqwest::Response;

use crate::error::ApiError;
use crate::models::{MessageParam, MessageRequest, MessageResponse, StreamEvent};

const BASE_URL: &str = "https://api.anthropic.com/v1/messages";

pub struct Client {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl Client {
    pub fn new(api_key: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            base_url: BASE_URL.to_string(),
        }
    }
    pub async fn send(
        &self,
        model: &str,
        messages: &[MessageParam],
    ) -> Result<MessageResponse, ApiError> {
        let request_body = MessageRequest {
            model: model.to_owned(),
            max_tokens: 1024,
            messages: messages.to_vec(),
            system: Some("".to_string()),
            stream: false,
        };
        let response = self
            .http
            .post(&self.base_url)
            .json(&request_body)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            return Err(ApiError::Status {
                status: status.as_u16(),
                body,
            });
        }
        let parsed = response.json::<MessageResponse>().await?;
        Ok(parsed)
    }

    /// streaming method
    pub async fn stream(
        &self,
        model: &str,
        messages: &[MessageParam],
    ) -> Result<impl Stream<Item = Result<StreamEvent, ApiError>> + use<>, ApiError> {
        let request_body = MessageRequest {
            model: model.to_owned(),
            max_tokens: 1024,
            messages: messages.to_vec(),
            system: Some("".to_string()),
            stream: true,
        };
        let response = self
            .http
            .post(&self.base_url)
            .json(&request_body)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            return Err(ApiError::Status {
                status: status.as_u16(),
                body,
            });
        }
        let bytes = Response::bytes_stream(response).map(|chunk| chunk.map_err(ApiError::from));
        Ok(parse_sse(bytes))
    }
}

/// Frame a raw SSE byte stream into modeled [`StreamEvent`]s.
///
/// Server-sent events are separated by a blank line; each frame's `data:`
/// payload is parsed as a `StreamEvent`. Comment lines, the `[DONE]` sentinel,
/// and unmodeled event types are skipped. Framing state (a partial event split
/// across byte chunks) is carried in the buffer between polls.
fn parse_sse(
    bytes: impl Stream<Item = Result<Bytes, ApiError>>,
) -> impl Stream<Item = Result<StreamEvent, ApiError>> {
    stream::unfold(
        (Box::pin(bytes), Vec::<u8>::new()),
        |(mut bytes, mut buf)| async move {
            loop {
                // Emit the next complete event already sitting in the buffer.
                while let Some(pos) = find(&buf, b"\n\n") {
                    let event = parse_frame(&buf[..pos]);
                    buf.drain(..pos + 2);
                    if let Some(event) = event {
                        return Some((Ok(event), (bytes, buf)));
                    }
                }
                // Otherwise pull more bytes and try the buffer again.
                match bytes.next().await {
                    Some(Ok(chunk)) => buf.extend_from_slice(&chunk),
                    Some(Err(e)) => return Some((Err(e), (bytes, buf))),
                    None => return None,
                }
            }
        },
    )
}

/// Extract the first modeled event from a single SSE frame, if any.
fn parse_frame(frame: &[u8]) -> Option<StreamEvent> {
    let text = std::str::from_utf8(frame).ok()?;
    for line in text.lines() {
        let Some(json) = line.strip_prefix("data: ") else {
            continue; // skip "event:" and comment lines
        };
        if json.trim() == "[DONE]" {
            continue;
        }
        if let Ok(event) = serde_json::from_str(json) {
            return Some(event);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ContentDelta;

    /// A complete SSE message: an `event:` line, a `data:` payload, blank-line terminator.
    fn sse(json: &str) -> String {
        format!("event: x\ndata: {json}\n\n")
    }

    const TEXT_DELTA: &str =
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#;

    fn text_event() -> StreamEvent {
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta { text: "Hi".into() },
        }
    }

    // ----- parse_frame: synchronous, one frame in -> at most one event out -----

    #[test]
    fn frame_parses_data_line_and_ignores_event_line() {
        let frame = format!("event: content_block_delta\ndata: {TEXT_DELTA}");
        assert_eq!(parse_frame(frame.as_bytes()), Some(text_event()));
    }

    #[test]
    fn frame_skips_done_sentinel_and_comment_lines() {
        assert_eq!(parse_frame(b"data: [DONE]"), None);
        assert_eq!(parse_frame(b": this is a comment"), None);
    }

    #[test]
    fn frame_skips_unmodeled_and_malformed_payloads() {
        assert_eq!(parse_frame(br#"data: {"type":"ping"}"#), None); // unmodeled event type
        assert_eq!(parse_frame(b"data: not json"), None); // malformed JSON
        assert_eq!(parse_frame(b"\xff\xfe invalid utf8"), None); // not valid UTF-8
    }

    // ----- parse_sse: async framing over a chunked byte stream -----

    /// Drive a sequence of byte chunks through `parse_sse` and collect the events.
    async fn run(chunks: Vec<Result<Bytes, ApiError>>) -> Vec<Result<StreamEvent, ApiError>> {
        parse_sse(stream::iter(chunks)).collect().await
    }

    #[tokio::test]
    async fn emits_every_event_in_a_single_chunk() {
        let body = format!("{}{}", sse(TEXT_DELTA), sse(TEXT_DELTA));
        let events = run(vec![Ok(Bytes::from(body))]).await;

        let texts: Vec<_> = events.into_iter().map(|e| e.unwrap()).collect();
        assert_eq!(texts, vec![text_event(), text_event()]);
    }

    #[tokio::test]
    async fn reassembles_an_event_split_across_chunk_boundaries() {
        // Split one SSE message at an arbitrary mid-JSON byte; the buffer must stitch it back.
        let full = sse(TEXT_DELTA);
        let (head, tail) = full.split_at(40);
        let events = run(vec![
            Ok(Bytes::from(head.to_owned())),
            Ok(Bytes::from(tail.to_owned())),
        ])
        .await;

        let texts: Vec<_> = events.into_iter().map(|e| e.unwrap()).collect();
        assert_eq!(texts, vec![text_event()]);
    }

    #[tokio::test]
    async fn skips_non_text_events_between_text_deltas() {
        let body = format!(
            "{}{}{}",
            sse(r#"{"type":"message_start"}"#),
            sse(TEXT_DELTA),
            sse(r#"{"type":"message_stop"}"#),
        );
        let events = run(vec![Ok(Bytes::from(body))]).await;

        // message_start / message_stop are modeled but carry no text; only the delta matters here.
        let modeled: Vec<_> = events.into_iter().map(|e| e.unwrap()).collect();
        assert_eq!(
            modeled,
            vec![
                StreamEvent::MessageStart {},
                text_event(),
                StreamEvent::MessageStop {},
            ]
        );
    }

    #[tokio::test]
    async fn forwards_a_transport_error_from_the_byte_stream() {
        let chunks = vec![
            Ok(Bytes::from(sse(TEXT_DELTA))),
            Err(ApiError::Status {
                status: 500,
                body: "boom".into(),
            }),
        ];
        let events = run(chunks).await;

        assert_eq!(events.len(), 2);
        assert!(events[0].is_ok());
        assert!(matches!(events[1], Err(ApiError::Status { status: 500, .. })));
    }
}
