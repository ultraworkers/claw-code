use std::collections::BTreeMap;
use std::env;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const ROSTER_VERSION: u32 = 1;

#[derive(Debug)]
pub enum AgentSupervisorError {
    ConfigDirUnavailable,
    Io(std::io::Error),
    Json(serde_json::Error),
    JobNotFound(String),
}

impl Display for AgentSupervisorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigDirUnavailable => {
                write!(
                    f,
                    "agent_supervisor_config_unavailable: set CLAUDE_CONFIG_DIR or HOME"
                )
            }
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::JobNotFound(id) => write!(f, "agent_job_not_found: {id}"),
        }
    }
}

impl std::error::Error for AgentSupervisorError {}

impl From<std::io::Error> for AgentSupervisorError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for AgentSupervisorError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Clone)]
pub struct AgentSupervisor {
    config_dir: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct AgentListFilter {
    pub cwd: Option<PathBuf>,
    pub include_all: bool,
}

#[derive(Debug, Clone)]
pub struct AgentJobCreate {
    pub cwd: PathBuf,
    pub kind: AgentJobKind,
    pub prompt: Option<String>,
    pub command: Option<String>,
    pub name: Option<String>,
    pub model: Option<String>,
    pub agent: Option<String>,
    pub permission_mode: Option<String>,
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentJobKind {
    Claude,
    Exec,
    Interactive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentJobState {
    Working,
    Blocked,
    Done,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentJobRecord {
    pub id: String,
    pub kind: AgentJobKind,
    pub cwd: String,
    pub started_at: String,
    pub updated_at: String,
    pub state: AgentJobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_for: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRoster {
    pub version: u32,
    pub updated_at: String,
    pub sessions: Vec<AgentRosterEntry>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRosterEntry {
    pub id: String,
    pub state: AgentJobState,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDaemonStatus {
    pub reachable: bool,
    pub version: String,
    pub config_dir: String,
    pub socket_dir: String,
    pub roster_path: String,
    pub log_path: String,
    pub live_sessions: usize,
    pub worker_count: usize,
}

impl AgentSupervisor {
    pub fn from_default_config() -> Result<Self, AgentSupervisorError> {
        Ok(Self {
            config_dir: default_claude_config_dir()?,
        })
    }

    #[must_use]
    pub fn from_config_dir(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_dir: config_dir.into(),
        }
    }

    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    #[must_use]
    pub fn jobs_dir(&self) -> PathBuf {
        self.config_dir.join("jobs")
    }

    #[must_use]
    pub fn daemon_dir(&self) -> PathBuf {
        self.config_dir.join("daemon")
    }

    #[must_use]
    pub fn roster_path(&self) -> PathBuf {
        self.daemon_dir().join("roster.json")
    }

    #[must_use]
    pub fn log_path(&self) -> PathBuf {
        self.config_dir.join("daemon.log")
    }

    #[must_use]
    pub fn job_dir(&self, id: &str) -> PathBuf {
        self.jobs_dir().join(id)
    }

    #[must_use]
    pub fn state_path(&self, id: &str) -> PathBuf {
        self.job_dir(id).join("state.json")
    }

    pub fn create_job(
        &self,
        request: AgentJobCreate,
    ) -> Result<AgentJobRecord, AgentSupervisorError> {
        fs::create_dir_all(self.jobs_dir())?;
        fs::create_dir_all(self.daemon_dir())?;
        let mut id = short_job_id();
        while self.state_path(&id).exists() {
            id = short_job_id();
        }
        let job_dir = self.job_dir(&id);
        fs::create_dir_all(job_dir.join("tmp"))?;
        let now = timestamp();
        let cwd = canonical_string(&request.cwd);
        let record = AgentJobRecord {
            id,
            kind: request.kind,
            cwd,
            started_at: now.clone(),
            updated_at: now,
            state: AgentJobState::Working,
            pid: None,
            status: Some("queued".to_string()),
            waiting_for: None,
            session_id: None,
            name: request.name,
            prompt: request.prompt,
            command: request.command,
            model: request.model,
            agent: request.agent,
            permission_mode: request.permission_mode,
            reasoning_effort: request.reasoning_effort,
            output_tail: None,
            exit_code: None,
            stopped_at: None,
            completed_at: None,
            extra: BTreeMap::new(),
        };
        self.save_job(&record)?;
        self.rewrite_roster()?;
        Ok(record)
    }

    pub fn list_jobs(
        &self,
        filter: &AgentListFilter,
    ) -> Result<Vec<AgentJobRecord>, AgentSupervisorError> {
        let entries = match fs::read_dir(self.jobs_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let cwd_filter = filter.cwd.as_ref().map(|cwd| canonical_string(cwd));
        let mut records = Vec::new();
        for entry in entries {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            let Ok(record) = self.load_job(&id) else {
                continue;
            };
            if let Some(cwd_filter) = &cwd_filter {
                if !record.cwd.starts_with(cwd_filter) {
                    continue;
                }
            }
            if !filter.include_all
                && !matches!(
                    record.state,
                    AgentJobState::Working | AgentJobState::Blocked
                )
                && record.pid.is_none()
            {
                continue;
            }
            records.push(record);
        }
        records.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then(left.id.cmp(&right.id))
        });
        Ok(records)
    }

    pub fn load_job(&self, id: &str) -> Result<AgentJobRecord, AgentSupervisorError> {
        let path = self.state_path(id);
        if !path.is_file() {
            return Err(AgentSupervisorError::JobNotFound(id.to_string()));
        }
        let contents = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    pub fn save_job(&self, record: &AgentJobRecord) -> Result<(), AgentSupervisorError> {
        let path = self.state_path(&record.id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(record)?)?;
        Ok(())
    }

    pub fn set_process(&self, id: &str, pid: u32) -> Result<AgentJobRecord, AgentSupervisorError> {
        self.update_job(id, |record| {
            record.pid = Some(pid);
            record.status = Some("running".to_string());
            record.state = AgentJobState::Working;
        })
    }

    pub fn set_session_id(
        &self,
        id: &str,
        session_id: impl Into<String>,
    ) -> Result<AgentJobRecord, AgentSupervisorError> {
        self.update_job(id, |record| {
            record.session_id = Some(session_id.into());
        })
    }

    pub fn append_log(&self, id: &str, text: &str) -> Result<AgentJobRecord, AgentSupervisorError> {
        let log_path = self.job_dir(id).join("output.log");
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut existing = fs::read_to_string(&log_path).unwrap_or_default();
        existing.push_str(text);
        fs::write(&log_path, &existing)?;
        self.update_job(id, |record| {
            record.output_tail = Some(tail(&existing, 8192));
        })
    }

    pub fn finish_job(
        &self,
        id: &str,
        exit_code: i32,
        output: Option<&str>,
    ) -> Result<AgentJobRecord, AgentSupervisorError> {
        self.update_job(id, |record| {
            record.pid = None;
            record.exit_code = Some(exit_code);
            record.completed_at = Some(timestamp());
            record.status = Some("exited".to_string());
            record.state = if exit_code == 0 {
                AgentJobState::Done
            } else {
                AgentJobState::Failed
            };
            if let Some(output) = output {
                record.output_tail = Some(tail(output, 8192));
            }
        })
    }

    pub fn stop_job(&self, id: &str) -> Result<AgentJobRecord, AgentSupervisorError> {
        self.update_job(id, |record| {
            record.pid = None;
            record.state = AgentJobState::Stopped;
            record.status = Some("stopped".to_string());
            record.stopped_at = Some(timestamp());
        })
    }

    pub fn respawn_job(&self, id: &str) -> Result<AgentJobRecord, AgentSupervisorError> {
        self.update_job(id, |record| {
            record.pid = None;
            record.state = AgentJobState::Working;
            record.status = Some("queued".to_string());
            record.stopped_at = None;
            record.completed_at = None;
            record.exit_code = None;
        })
    }

    pub fn remove_job(&self, id: &str) -> Result<(), AgentSupervisorError> {
        let path = self.job_dir(id);
        if !path.exists() {
            return Err(AgentSupervisorError::JobNotFound(id.to_string()));
        }
        fs::remove_dir_all(path)?;
        self.rewrite_roster()?;
        Ok(())
    }

    pub fn read_logs(&self, id: &str) -> Result<String, AgentSupervisorError> {
        let state = self.load_job(id)?;
        let log_path = self.job_dir(id).join("output.log");
        Ok(fs::read_to_string(log_path).unwrap_or_else(|_| {
            state
                .output_tail
                .unwrap_or_else(|| "No output captured for this session yet.".to_string())
        }))
    }

    pub fn daemon_status(
        &self,
        version: impl Into<String>,
    ) -> Result<AgentDaemonStatus, AgentSupervisorError> {
        let filter = AgentListFilter {
            cwd: None,
            include_all: false,
        };
        let live_sessions = self.list_jobs(&filter)?.len();
        Ok(AgentDaemonStatus {
            reachable: self.roster_path().is_file() || live_sessions > 0,
            version: version.into(),
            config_dir: self.config_dir.display().to_string(),
            socket_dir: self.daemon_dir().display().to_string(),
            roster_path: self.roster_path().display().to_string(),
            log_path: self.log_path().display().to_string(),
            live_sessions,
            worker_count: 0,
        })
    }

    pub fn rewrite_roster(&self) -> Result<AgentRoster, AgentSupervisorError> {
        fs::create_dir_all(self.daemon_dir())?;
        let jobs = self.list_jobs(&AgentListFilter {
            cwd: None,
            include_all: true,
        })?;
        let now = timestamp();
        let roster = AgentRoster {
            version: ROSTER_VERSION,
            updated_at: now,
            sessions: jobs
                .iter()
                .map(|job| AgentRosterEntry {
                    id: job.id.clone(),
                    state: job.state.clone(),
                    cwd: job.cwd.clone(),
                    pid: job.pid,
                    name: job.name.clone(),
                    started_at: Some(job.started_at.clone()),
                    updated_at: Some(job.updated_at.clone()),
                    extra: BTreeMap::new(),
                })
                .collect(),
            extra: BTreeMap::new(),
        };
        fs::write(self.roster_path(), serde_json::to_string_pretty(&roster)?)?;
        Ok(roster)
    }

    fn update_job(
        &self,
        id: &str,
        update: impl FnOnce(&mut AgentJobRecord),
    ) -> Result<AgentJobRecord, AgentSupervisorError> {
        let mut record = self.load_job(id)?;
        update(&mut record);
        record.updated_at = timestamp();
        self.save_job(&record)?;
        self.rewrite_roster()?;
        Ok(record)
    }
}

pub fn default_claude_config_dir() -> Result<PathBuf, AgentSupervisorError> {
    if let Ok(path) = env::var("CLAUDE_CONFIG_DIR") {
        return Ok(PathBuf::from(path));
    }
    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".claude"));
    }
    Err(AgentSupervisorError::ConfigDirUnavailable)
}

fn canonical_string(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn short_job_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id() as u128;
    format!("{:08x}", (nanos ^ pid) & 0xffff_ffff)
}

fn tail(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    chars[chars.len().saturating_sub(max_chars)..]
        .iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("agent-supervisor-{label}-{nanos}"))
    }

    #[test]
    fn creates_job_state_and_roster_under_config_dir() {
        let config = temp_dir("create");
        let workspace = temp_dir("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        let supervisor = AgentSupervisor::from_config_dir(&config);

        let job = supervisor
            .create_job(AgentJobCreate {
                cwd: workspace.clone(),
                kind: AgentJobKind::Claude,
                prompt: Some("fix the parser".to_string()),
                command: None,
                name: Some("parser-fix".to_string()),
                model: Some("sonnet".to_string()),
                agent: None,
                permission_mode: Some("manual".to_string()),
                reasoning_effort: Some("medium".to_string()),
            })
            .expect("job");

        assert!(supervisor.state_path(&job.id).is_file());
        assert!(supervisor.roster_path().is_file());
        let listed = supervisor
            .list_jobs(&AgentListFilter {
                cwd: Some(workspace),
                include_all: false,
            })
            .expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name.as_deref(), Some("parser-fix"));

        let _ = fs::remove_dir_all(config);
    }

    #[test]
    fn lifecycle_updates_state_and_default_listing_hides_finished_jobs() {
        let config = temp_dir("lifecycle");
        let workspace = temp_dir("workspace");
        fs::create_dir_all(&workspace).expect("workspace");
        let supervisor = AgentSupervisor::from_config_dir(&config);
        let job = supervisor
            .create_job(AgentJobCreate {
                cwd: workspace,
                kind: AgentJobKind::Exec,
                prompt: None,
                command: Some("echo hi".to_string()),
                name: None,
                model: None,
                agent: None,
                permission_mode: None,
                reasoning_effort: None,
            })
            .expect("job");

        supervisor.set_process(&job.id, 1234).expect("pid");
        supervisor.append_log(&job.id, "hello\n").expect("log");
        let done = supervisor
            .finish_job(&job.id, 0, Some("hello\n"))
            .expect("done");
        assert_eq!(done.state, AgentJobState::Done);
        assert_eq!(supervisor.read_logs(&job.id).expect("logs"), "hello\n");
        assert!(supervisor
            .list_jobs(&AgentListFilter::default())
            .expect("default")
            .is_empty());
        assert_eq!(
            supervisor
                .list_jobs(&AgentListFilter {
                    cwd: None,
                    include_all: true,
                })
                .expect("all")
                .len(),
            1
        );

        supervisor.respawn_job(&job.id).expect("respawn");
        assert_eq!(
            supervisor.load_job(&job.id).expect("load").state,
            AgentJobState::Working
        );
        supervisor.remove_job(&job.id).expect("remove");
        assert!(matches!(
            supervisor.load_job(&job.id),
            Err(AgentSupervisorError::JobNotFound(_))
        ));

        let _ = fs::remove_dir_all(config);
    }
}
