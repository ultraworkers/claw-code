use crate::error::ApiError;
use crate::prompt_cache::{PromptCache, PromptCacheRecord, PromptCacheStats};
use crate::providers::anthropic::{self, AnthropicClient, AuthSource};
use crate::providers::openai_compat;
use crate::providers::openai_compat::{OpenAiCompatClient, OpenAiCompatConfig};
use crate::providers::{self, ProviderKind};
use crate::types::{MessageRequest, MessageResponse, StreamEvent};

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum ProviderClient {
    Anthropic(AnthropicClient),
    OpenAi(OpenAiCompatClient),
}

impl ProviderClient {
    pub fn from_model(model: &str) -> Result<Self, ApiError> {
        Self::from_model_with_anthropic_auth(model, None)
    }

    pub fn from_model_with_anthropic_auth(
        model: &str,
        anthropic_auth: Option<AuthSource>,
    ) -> Result<Self, ApiError> {
        let resolved_model = providers::resolve_model_alias(model);
        match providers::detect_provider_kind(&resolved_model) {
            ProviderKind::Anthropic => Ok(Self::Anthropic(match anthropic_auth {
                Some(auth) => AnthropicClient::from_auth(auth),
                None => AnthropicClient::from_env()?,
            })),
            ProviderKind::OpenAi => Ok(Self::OpenAi(OpenAiCompatClient::from_env(
                OpenAiCompatConfig::openai(),
            )?)),
        }
    }

    #[must_use]
    pub const fn provider_kind(&self) -> ProviderKind {
        match self {
            Self::Anthropic(_) => ProviderKind::Anthropic,
            Self::OpenAi(_) => ProviderKind::OpenAi,
        }
    }

    #[must_use]
    pub fn with_prompt_cache(self, prompt_cache: PromptCache) -> Self {
        match self {
            Self::Anthropic(client) => Self::Anthropic(client.with_prompt_cache(prompt_cache)),
            other => other,
        }
    }

    #[must_use]
    pub fn prompt_cache_stats(&self) -> Option<PromptCacheStats> {
        match self {
            Self::Anthropic(client) => client.prompt_cache_stats(),
            Self::OpenAi(_) => None,
        }
    }

    #[must_use]
    pub fn take_last_prompt_cache_record(&self) -> Option<PromptCacheRecord> {
        match self {
            Self::Anthropic(client) => client.take_last_prompt_cache_record(),
            Self::OpenAi(_) => None,
        }
    }

    /// Enable incremental body serialisation (Anthropic only).
    #[must_use]
    pub fn with_incremental_body(self) -> Self {
        match self {
            Self::Anthropic(client) => Self::Anthropic(client.with_incremental_body()),
            other => other,
        }
    }

    pub async fn send_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageResponse, ApiError> {
        match self {
            Self::Anthropic(client) => client.send_message(request).await,
            Self::OpenAi(client) => client.send_message(request).await,
        }
    }

    pub async fn stream_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageStream, ApiError> {
        match self {
            Self::Anthropic(client) => client
                .stream_message(request)
                .await
                .map(MessageStream::Anthropic),
            Self::OpenAi(client) => client
                .stream_message(request)
                .await
                .map(MessageStream::OpenAiCompat),
        }
    }
}

#[derive(Debug)]
pub enum MessageStream {
    Anthropic(anthropic::MessageStream),
    OpenAiCompat(openai_compat::MessageStream),
}

impl MessageStream {
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Anthropic(stream) => stream.request_id(),
            Self::OpenAiCompat(stream) => stream.request_id(),
        }
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        match self {
            Self::Anthropic(stream) => stream.next_event().await,
            Self::OpenAiCompat(stream) => stream.next_event().await,
        }
    }
}

pub use anthropic::{
    oauth_token_is_expired, resolve_saved_oauth_token, resolve_startup_auth_source, OAuthTokenSet,
};
#[must_use]
pub fn read_base_url() -> String {
    anthropic::read_base_url()
}

#[cfg(test)]
mod tests {
    use crate::providers::{detect_provider_kind, resolve_model_alias, ProviderKind};

    #[test]
    fn resolves_existing_aliases() {
        assert_eq!(resolve_model_alias("opus"), "claude-opus-4-6");
    }

    #[test]
    fn provider_detection_prefers_model_family() {
        assert_eq!(
            detect_provider_kind("claude-sonnet-4-6"),
            ProviderKind::Anthropic
        );
    }
}
