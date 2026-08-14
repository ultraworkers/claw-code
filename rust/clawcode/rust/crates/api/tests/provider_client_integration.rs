use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

use api::{AuthSource, ProviderClient, ProviderKind};

#[test]
fn provider_client_uses_explicit_anthropic_auth_without_env_lookup() {
    let _lock = env_lock();
    let _anthropic_api_key = EnvVarGuard::set("ANTHROPIC_API_KEY", None);

    let client = ProviderClient::from_model_with_anthropic_auth(
        "claude-sonnet-4-6",
        Some(AuthSource::ApiKey("anthropic-test-key".to_string())),
    )
    .expect("explicit anthropic auth should avoid env lookup");

    assert_eq!(client.provider_kind(), ProviderKind::Anthropic);
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let original = std::env::var_os(key);
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
