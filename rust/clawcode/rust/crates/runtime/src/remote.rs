use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const DEFAULT_REMOTE_BASE_URL: &str = "https://api.anthropic.com";
pub const DEFAULT_SESSION_TOKEN_PATH: &str = "/run/ccr/session_token";
pub const DEFAULT_SYSTEM_CA_BUNDLE: &str = "/etc/ssl/certs/ca-certificates.crt";

pub const UPSTREAM_PROXY_ENV_KEYS: [&str; 8] = [
    "HTTPS_PROXY",
    "https_proxy",
    "NO_PROXY",
    "no_proxy",
    "SSL_CERT_FILE",
    "NODE_EXTRA_CA_CERTS",
    "REQUESTS_CA_BUNDLE",
    "CURL_CA_BUNDLE",
];

pub const NO_PROXY_HOSTS: [&str; 16] = [
    "localhost",
    "127.0.0.1",
    "::1",
    "169.254.0.0/16",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "anthropic.com",
    ".anthropic.com",
    "*.anthropic.com",
    "github.com",
    "api.github.com",
    "*.github.com",
    "*.githubusercontent.com",
    "registry.npmjs.org",
    "index.crates.io",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSessionContext {
    pub enabled: bool,
    pub session_id: Option<String>,
    pub base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamProxyBootstrap {
    pub remote: RemoteSessionContext,
    pub upstream_proxy_enabled: bool,
    pub token_path: PathBuf,
    pub ca_bundle_path: PathBuf,
    pub system_ca_path: PathBuf,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamProxyState {
    pub enabled: bool,
    pub proxy_url: Option<String>,
    pub ca_bundle_path: Option<PathBuf>,
    pub no_proxy: String,
}

impl RemoteSessionContext {
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_env_map(&env::vars().collect())
    }

    #[must_use]
    pub fn from_env_map(env_map: &BTreeMap<String, String>) -> Self {
        Self {
            enabled: env_truthy(env_map.get("CLAUDE_CODE_REMOTE")),
            session_id: env_map
                .get("CLAUDE_CODE_REMOTE_SESSION_ID")
                .filter(|value| !value.is_empty())
                .cloned(),
            base_url: env_map
                .get("ANTHROPIC_BASE_URL")
                .filter(|value| !value.is_empty())
                .cloned()
                .unwrap_or_else(|| DEFAULT_REMOTE_BASE_URL.to_string()),
        }
    }
}

impl UpstreamProxyBootstrap {
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_env_map(&env::vars().collect())
    }

    #[must_use]
    pub fn from_env_map(env_map: &BTreeMap<String, String>) -> Self {
        let remote = RemoteSessionContext::from_env_map(env_map);
        let token_path = env_map
            .get("CCR_SESSION_TOKEN_PATH")
            .filter(|value| !value.is_empty())
            .map_or_else(|| PathBuf::from(DEFAULT_SESSION_TOKEN_PATH), PathBuf::from);
        let system_ca_path = env_map
            .get("CCR_SYSTEM_CA_BUNDLE")
            .filter(|value| !value.is_empty())
            .map_or_else(|| PathBuf::from(DEFAULT_SYSTEM_CA_BUNDLE), PathBuf::from);
        let ca_bundle_path = env_map
            .get("CCR_CA_BUNDLE_PATH")
            .filter(|value| !value.is_empty())
            .map_or_else(default_ca_bundle_path, PathBuf::from);
        let token = read_token(&token_path).ok().flatten();

        Self {
            remote,
            upstream_proxy_enabled: env_truthy(env_map.get("CCR_UPSTREAM_PROXY_ENABLED")),
            token_path,
            ca_bundle_path,
            system_ca_path,
            token,
        }
    }

    #[must_use]
    pub fn should_enable(&self) -> bool {
        self.remote.enabled
            && self.upstream_proxy_enabled
            && self.remote.session_id.is_some()
            && self.token.is_some()
    }

    #[must_use]
    pub fn ws_url(&self) -> String {
        upstream_proxy_ws_url(&self.remote.base_url)
    }

    #[must_use]
    pub fn state_for_port(&self, port: u16) -> UpstreamProxyState {
        if !self.should_enable() {
            return UpstreamProxyState::disabled();
        }
        UpstreamProxyState {
            enabled: true,
            proxy_url: Some(format!("http://127.0.0.1:{port}")),
            ca_bundle_path: Some(self.ca_bundle_path.clone()),
            no_proxy: no_proxy_list(),
        }
    }
}

impl UpstreamProxyState {
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            proxy_url: None,
            ca_bundle_path: None,
            no_proxy: no_proxy_list(),
        }
    }

    #[must_use]
    pub fn subprocess_env(&self) -> BTreeMap<String, String> {
        if !self.enabled {
            return BTreeMap::new();
        }
        let Some(proxy_url) = &self.proxy_url else {
            return BTreeMap::new();
        };
        let Some(ca_bundle_path) = &self.ca_bundle_path else {
            return BTreeMap::new();
        };
        let ca_bundle_path = ca_bundle_path.to_string_lossy().into_owned();
        BTreeMap::from([
            ("HTTPS_PROXY".to_string(), proxy_url.clone()),
            ("https_proxy".to_string(), proxy_url.clone()),
            ("NO_PROXY".to_string(), self.no_proxy.clone()),
            ("no_proxy".to_string(), self.no_proxy.clone()),
            ("SSL_CERT_FILE".to_string(), ca_bundle_path.clone()),
            ("NODE_EXTRA_CA_CERTS".to_string(), ca_bundle_path.clone()),
            ("REQUESTS_CA_BUNDLE".to_string(), ca_bundle_path.clone()),
            ("CURL_CA_BUNDLE".to_string(), ca_bundle_path),
        ])
    }
}

pub fn read_token(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let token = contents.trim();
            if token.is_empty() {
                Ok(None)
            } else {
                Ok(Some(token.to_string()))
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[must_use]
pub fn upstream_proxy_ws_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let ws_base = if let Some(stripped) = base.strip_prefix("https://") {
        format!("wss://{stripped}")
    } else if let Some(stripped) = base.strip_prefix("http://") {
        format!("ws://{stripped}")
    } else {
        format!("wss://{base}")
    };
    format!("{ws_base}/v1/code/upstreamproxy/ws")
}

#[must_use]
pub fn no_proxy_list() -> String {
    let mut hosts = NO_PROXY_HOSTS.to_vec();
    hosts.extend(["pypi.org", "files.pythonhosted.org", "proxy.golang.org"]);
    hosts.join(",")
}

#[must_use]
pub fn inherited_upstream_proxy_env(
    env_map: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    if !(env_map.contains_key("HTTPS_PROXY") && env_map.contains_key("SSL_CERT_FILE")) {
        return BTreeMap::new();
    }
    UPSTREAM_PROXY_ENV_KEYS
        .iter()
        .filter_map(|key| {
            env_map
                .get(*key)
                .map(|value| ((*key).to_string(), value.clone()))
        })
        .collect()
}

fn default_ca_bundle_path() -> PathBuf {
    env::var_os("HOME")
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join(".ccr")
        .join("ca-bundle.crt")
}

fn env_truthy(value: Option<&String>) -> bool {
    value.is_some_and(|raw| {
        matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        inherited_upstream_proxy_env, no_proxy_list, read_token, upstream_proxy_ws_url,
        RemoteSessionContext, UpstreamProxyBootstrap,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-remote-{nanos}"))
    }

    #[test]
    fn remote_context_reads_env_state() {
        let env = BTreeMap::from([
            ("CLAUDE_CODE_REMOTE".to_string(), "true".to_string()),
            (
                "CLAUDE_CODE_REMOTE_SESSION_ID".to_string(),
                "session-123".to_string(),
            ),
            (
                "ANTHROPIC_BASE_URL".to_string(),
                "https://remote.test".to_string(),
            ),
        ]);
        let context = RemoteSessionContext::from_env_map(&env);
        assert!(context.enabled);
        assert_eq!(context.session_id.as_deref(), Some("session-123"));
        assert_eq!(context.base_url, "https://remote.test");
    }

    #[test]
    fn bootstrap_fails_open_when_token_or_session_is_missing() {
        let env = BTreeMap::from([
            ("CLAUDE_CODE_REMOTE".to_string(), "1".to_string()),
            ("CCR_UPSTREAM_PROXY_ENABLED".to_string(), "true".to_string()),
        ]);
        let bootstrap = UpstreamProxyBootstrap::from_env_map(&env);
        assert!(!bootstrap.should_enable());
        assert!(!bootstrap.state_for_port(8080).enabled);
    }

    #[test]
    fn bootstrap_derives_proxy_state_and_env() {
        let root = temp_dir();
        let token_path = root.join("session_token");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(&token_path, "secret-token\n").expect("write token");

        let env = BTreeMap::from([
            ("CLAUDE_CODE_REMOTE".to_string(), "1".to_string()),
            ("CCR_UPSTREAM_PROXY_ENABLED".to_string(), "true".to_string()),
            (
                "CLAUDE_CODE_REMOTE_SESSION_ID".to_string(),
                "session-123".to_string(),
            ),
            (
                "ANTHROPIC_BASE_URL".to_string(),
                "https://remote.test".to_string(),
            ),
            (
                "CCR_SESSION_TOKEN_PATH".to_string(),
                token_path.to_string_lossy().into_owned(),
            ),
            (
                "CCR_CA_BUNDLE_PATH".to_string(),
                root.join("ca-bundle.crt").to_string_lossy().into_owned(),
            ),
        ]);

        let bootstrap = UpstreamProxyBootstrap::from_env_map(&env);
        assert!(bootstrap.should_enable());
        assert_eq!(bootstrap.token.as_deref(), Some("secret-token"));
        assert_eq!(
            bootstrap.ws_url(),
            "wss://remote.test/v1/code/upstreamproxy/ws"
        );

        let state = bootstrap.state_for_port(9443);
        assert!(state.enabled);
        let env = state.subprocess_env();
        assert_eq!(
            env.get("HTTPS_PROXY").map(String::as_str),
            Some("http://127.0.0.1:9443")
        );
        assert_eq!(
            env.get("SSL_CERT_FILE").map(String::as_str),
            Some(root.join("ca-bundle.crt").to_string_lossy().as_ref())
        );

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn token_reader_trims_and_handles_missing_files() {
        let root = temp_dir();
        fs::create_dir_all(&root).expect("temp dir");
        let token_path = root.join("session_token");
        fs::write(&token_path, " abc123 \n").expect("write token");
        assert_eq!(
            read_token(&token_path).expect("read token").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            read_token(&root.join("missing")).expect("missing token"),
            None
        );
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn inherited_proxy_env_requires_proxy_and_ca() {
        let env = BTreeMap::from([
            (
                "HTTPS_PROXY".to_string(),
                "http://127.0.0.1:8888".to_string(),
            ),
            (
                "SSL_CERT_FILE".to_string(),
                "/tmp/ca-bundle.crt".to_string(),
            ),
            ("NO_PROXY".to_string(), "localhost".to_string()),
        ]);
        let inherited = inherited_upstream_proxy_env(&env);
        assert_eq!(inherited.len(), 3);
        assert_eq!(
            inherited.get("NO_PROXY").map(String::as_str),
            Some("localhost")
        );
        assert!(inherited_upstream_proxy_env(&BTreeMap::new()).is_empty());
    }

    #[test]
    fn helper_outputs_match_expected_shapes() {
        assert_eq!(
            upstream_proxy_ws_url("http://localhost:3000/"),
            "ws://localhost:3000/v1/code/upstreamproxy/ws"
        );
        assert!(no_proxy_list().contains("anthropic.com"));
        assert!(no_proxy_list().contains("github.com"));
    }
}
