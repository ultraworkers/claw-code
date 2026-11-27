//! Optional outbound HTTP header provider for model requests.
//!
//! Some hosts must annotate each model request with routing, quota, or
//! cache-affinity metadata that only the caller can compute. A client opts in
//! with [`crate::providers::anthropic::AnthropicClient::with_extra_headers_provider`]
//! or [`crate::providers::openai_compat::OpenAiCompatClient::with_extra_headers_provider`];
//! clients without a provider send byte-for-byte the same headers as before.
//!
//! The provider runs in-process and receives the exact JSON body that is about
//! to be sent. It must treat the body as read-only: the hook can only add
//! headers, never change the payload.

use std::fmt;
use std::sync::Arc;

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::RequestBuilder;
use serde_json::Value;

/// Facts the host may use when deriving headers.
pub struct ExtraHeadersContext<'a> {
    /// Provider label reported by the client (`"Anthropic"`, `"OpenAI"`, ...).
    pub provider: &'a str,
    /// Model id as it appears in the request body.
    pub model: &'a str,
    /// The JSON body that will be sent with this request. Read-only.
    pub body: &'a Value,
}

/// Returns `(name, value)` pairs to add to one outbound request.
pub type ExtraHeadersProvider =
    dyn Fn(&ExtraHeadersContext<'_>) -> Vec<(String, String)> + Send + Sync;

/// Headers owned by the transport. A provider cannot override them.
const RESERVED_HEADERS: &[&str] = &[
    "accept",
    "accept-encoding",
    "anthropic-beta",
    "anthropic-version",
    "authorization",
    "connection",
    "content-length",
    "content-type",
    "host",
    "transfer-encoding",
    "user-agent",
    "x-api-key",
];

/// A header provider attached to one client.
///
/// Cloning a client shares the provider; the closure must be `Send + Sync`.
#[derive(Clone)]
pub struct ExtraHeadersHook {
    provider: Arc<ExtraHeadersProvider>,
}

impl ExtraHeadersHook {
    #[must_use]
    pub fn new(
        provider: impl Fn(&ExtraHeadersContext<'_>) -> Vec<(String, String)> + Send + Sync + 'static,
    ) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    /// Adds the provider's headers to one outbound model request.
    ///
    /// Invalid names or values, and headers the transport owns, are dropped: a
    /// broken provider must never fail a model call or rewrite credentials.
    pub fn apply(
        &self,
        mut builder: RequestBuilder,
        context: &ExtraHeadersContext<'_>,
    ) -> RequestBuilder {
        for (name, value) in (self.provider)(context) {
            let Ok(name) = HeaderName::from_bytes(name.as_bytes()) else {
                continue;
            };
            if RESERVED_HEADERS.contains(&name.as_str()) {
                continue;
            }
            let Ok(value) = HeaderValue::from_str(&value) else {
                continue;
            };
            builder = builder.header(name, value);
        }
        builder
    }
}

impl fmt::Debug for ExtraHeadersHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtraHeadersHook").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn context<'a>(body: &'a Value) -> ExtraHeadersContext<'a> {
        ExtraHeadersContext {
            provider: "Anthropic",
            model: "claude-sonnet-4",
            body,
        }
    }

    #[test]
    fn apply_keeps_valid_headers_and_drops_the_rest() {
        let body = json!({"messages": []});
        let hook = ExtraHeadersHook::new(|ctx: &ExtraHeadersContext<'_>| {
            vec![
                (
                    "x-route".to_string(),
                    format!("{}/{}", ctx.provider, ctx.model),
                ),
                ("authorization".to_string(), "Bearer nope".to_string()),
                ("anthropic-version".to_string(), "2099-01-01".to_string()),
                ("anthropic-beta".to_string(), "none".to_string()),
                ("bad header".to_string(), "dropped".to_string()),
                ("x-blank".to_string(), "bad\r\nvalue".to_string()),
            ]
        });

        let request = hook
            .apply(
                reqwest::Client::new().post("http://127.0.0.1/"),
                &context(&body),
            )
            .build()
            .expect("request builds");

        assert_eq!(
            request
                .headers()
                .get("x-route")
                .and_then(|value| value.to_str().ok()),
            Some("Anthropic/claude-sonnet-4")
        );
        assert!(request.headers().get("authorization").is_none());
        assert!(request.headers().get("anthropic-version").is_none());
        assert!(request.headers().get("anthropic-beta").is_none());
        assert!(request.headers().get("bad header").is_none());
        assert!(request.headers().get("x-blank").is_none());
    }
}
