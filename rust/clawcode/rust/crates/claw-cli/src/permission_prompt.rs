use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

// Keyboard input read via stdin::read_line — no raw mode, no screen clearing.
// Writing to stderr preserves the terminal scrollback from corruption.

use runtime::boundary::{
    ApprovedRoot, ApprovedRootsFile, BoundaryDecision, Prompter, PrompterError,
};

/// Messages from the main thread (conversation runtime) to the UI thread.
pub enum UiMessage {
    BoundaryPrompt(BoundaryPromptRequest),
    Shutdown,
}

/// A pending boundary prompt awaiting user decision on the UI thread.
pub struct BoundaryPromptRequest {
    pub id: u64,
    pub path: PathBuf,
    pub workspace: PathBuf,
    pub reply_tx: mpsc::Sender<BoundaryDecision>,
    pub cancel_flag: Arc<AtomicBool>,
}

/// Production prompter that communicates with a dedicated UI thread
/// via `mpsc` channels. The UI thread renders a crossterm overlay popup
/// and awaits keyboard input.
pub struct ChannelPrompter {
    ui_tx: mpsc::Sender<UiMessage>,
    is_tty: bool,
    next_id: std::sync::atomic::AtomicU64,
    session_approved: Arc<Mutex<BTreeSet<ApprovedRoot>>>,
    session_denied: Arc<Mutex<BTreeSet<ApprovedRoot>>>,
    pub user_typed: Arc<Mutex<BTreeSet<ApprovedRoot>>>,
    approved_roots_file: Mutex<ApprovedRootsFile>,
    timeout: Duration,
}

impl ChannelPrompter {
    pub fn new(ui_tx: mpsc::Sender<UiMessage>, is_tty: bool) -> Self {
        let approved_roots = ApprovedRootsFile::load().unwrap_or_default();
        Self {
            ui_tx,
            is_tty,
            next_id: std::sync::atomic::AtomicU64::new(1),
            session_approved: Arc::new(Mutex::new(BTreeSet::new())),
            session_denied: Arc::new(Mutex::new(BTreeSet::new())),
            user_typed: Arc::new(Mutex::new(BTreeSet::new())),
            approved_roots_file: Mutex::new(approved_roots),
            timeout: Duration::from_secs(60),
        }
    }

    fn is_parent_approved(set: &BTreeSet<ApprovedRoot>, path: &Path) -> bool {
        let parent = path.parent().unwrap_or(path);
        set.iter().any(|root| parent.starts_with(root.as_path()))
    }

}

impl Prompter for ChannelPrompter {
    fn ask(&self, path: &Path, workspace: &Path) -> Result<BoundaryDecision, PrompterError> {
        // (1) Non-TTY → Deny immediately
        if !self.is_tty {
            return Err(PrompterError::NoTty);
        }

        let simplified = dunce::simplified(path).to_path_buf();
        let parent = simplified.parent().unwrap_or(&simplified).to_path_buf();

        // (2) Check session_denied
        {
            let denied = self.session_denied.lock().map_err(|_| PrompterError::NoTty)?;
            if Self::is_parent_approved(&denied, &simplified) {
                return Err(PrompterError::NoTty);
            }
        }

        // (3) Check user_typed
        {
            let typed = self.user_typed.lock().map_err(|_| PrompterError::NoTty)?;
            if Self::is_parent_approved(&typed, &simplified) {
                return Ok(BoundaryDecision::AllowAlways);
            }
        }

        // (4) Check session_approved
        {
            let approved = self.session_approved.lock().map_err(|_| PrompterError::NoTty)?;
            if Self::is_parent_approved(&approved, &simplified) {
                return Ok(BoundaryDecision::AllowAlways);
            }
        }

        // (5) Check permanent approvals
        {
            let perm = self
                .approved_roots_file
                .lock()
                .map_err(|_| PrompterError::NoTty)?;
            if Self::is_parent_approved(&perm.roots, &simplified) {
                return Ok(BoundaryDecision::AllowAlways);
            }
        }

        // (6) Need to prompt — create oneshot channel
        let (reply_tx, reply_rx) = mpsc::channel();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let cancel_flag = Arc::new(AtomicBool::new(false));

        self.ui_tx
            .send(UiMessage::BoundaryPrompt(BoundaryPromptRequest {
                id,
                path: simplified.clone(),
                workspace: workspace.to_path_buf(),
                reply_tx,
                cancel_flag: cancel_flag.clone(),
            }))
            .map_err(|_| PrompterError::NoTty)?;

        // (7) Wait with timeout
        let result = reply_rx.recv_timeout(self.timeout);
        // Signal cancellation to the UI thread so it can close any pending prompt
        cancel_flag.store(true, Ordering::SeqCst);
        match result {
            Ok(BoundaryDecision::AllowOnce) => Ok(BoundaryDecision::AllowOnce),
            Ok(BoundaryDecision::AllowAlways) => {
                if let Ok(mut set) = self.session_approved.lock() {
                    set.insert(ApprovedRoot::new(parent));
                }
                Ok(BoundaryDecision::AllowAlways)
            }
            // AllowPermanent was removed — use AllowAlways instead
            Ok(BoundaryDecision::Deny) => {
                if let Ok(mut set) = self.session_denied.lock() {
                    set.insert(ApprovedRoot::new(parent));
                }
                Err(PrompterError::NoTty)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Err(PrompterError::Timeout(self.timeout)),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(PrompterError::NoTty),
        }
    }
}

/// Run the UI event loop in a dedicated thread.
/// Reads from `rx`, renders popups for `BoundaryPrompt` messages, and
/// sends user decisions back via `oneshot::Sender`.
pub fn run_ui_thread(rx: mpsc::Receiver<UiMessage>) {
    for msg in rx {
        match msg {
            UiMessage::Shutdown => break,
            UiMessage::BoundaryPrompt(request) => {
                handle_prompt(request);
            }
        }
    }
}

fn handle_prompt(request: BoundaryPromptRequest) {
    let mut stderr = io::stderr();

    let prompt = format!(
        "\nclaw: access {} (outside workspace {})\n\
         [o]nce / [a]lways / [d]eny: ",
        request.path.display(),
        request.workspace.display(),
    );

    let _ = write!(stderr, "{}", prompt);
    let _ = stderr.flush();

    let mut input = String::new();
    loop {
        if request.cancel_flag.load(Ordering::SeqCst) {
            break;
        }

        input.clear();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let decision = match input.trim().to_lowercase().as_str() {
                    "o" | "once" => Some(BoundaryDecision::AllowOnce),
                    "a" | "always" => Some(BoundaryDecision::AllowAlways),
                    "d" | "deny" => Some(BoundaryDecision::Deny),
                    _ => {
                        let _ = write!(stderr, "[o]nce / [a]lways / [d]eny: ");
                        let _ = stderr.flush();
                        None
                    }
                };
                if let Some(decision) = decision {
                    let _ = request.reply_tx.send(decision);
                    break;
                }
            }
            Err(_) => break,
        }
    }
}
