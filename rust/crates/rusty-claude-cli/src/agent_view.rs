//! Agent View — multi-session monitoring dashboard.

use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    WaitingForInput,
    Done,
    Failed,
    Cancelled,
}

impl AgentStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            AgentStatus::Running => "⬢",
            AgentStatus::WaitingForInput => "◐",
            AgentStatus::Done => "✓",
            AgentStatus::Failed => "✗",
            AgentStatus::Cancelled => "⊘",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentSession {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub model: String,
    pub turn_count: u32,
    pub last_message: String,
    pub working_dir: String,
    pub started_at: Instant,
    pub task_subject: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Status,
    Name,
    Started,
    TurnCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterState {
    All,
    Running,
    Done,
    Failed,
}

pub struct AgentView {
    pub sessions: Vec<AgentSession>,
    pub sort_by: SortField,
    pub filter: FilterState,
    pub selected: usize,
    pub active: bool,
}

impl AgentView {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            sort_by: SortField::Status,
            filter: FilterState::All,
            selected: 0,
            active: false,
        }
    }

    pub fn open(&mut self) {
        self.active = true;
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.active = false;
    }

    pub fn update_session(&mut self, session: AgentSession) {
        if let Some(existing) = self.sessions.iter_mut().find(|s| s.id == session.id) {
            *existing = session;
        } else {
            self.sessions.push(session);
        }
    }

    pub fn remove_session(&mut self, id: &str) {
        self.sessions.retain(|s| s.id != id);
    }

    pub fn filtered_sessions(&self) -> Vec<&AgentSession> {
        let mut sessions: Vec<&AgentSession> = self
            .sessions
            .iter()
            .filter(|s| match self.filter {
                FilterState::All => true,
                FilterState::Running => s.status == AgentStatus::Running,
                FilterState::Done => s.status == AgentStatus::Done,
                FilterState::Failed => s.status == AgentStatus::Failed,
            })
            .collect();
        sessions.sort_by(|a, b| match self.sort_by {
            SortField::Status => a.status.icon().cmp(b.status.icon()),
            SortField::Name => a.name.cmp(&b.name),
            SortField::Started => a.started_at.cmp(&b.started_at),
            SortField::TurnCount => a.turn_count.cmp(&b.turn_count),
        });
        sessions
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_sessions().len();
        if count > 0 {
            self.selected = (self.selected + 1) % count;
        }
    }

    pub fn select_prev(&mut self) {
        let count = self.filtered_sessions().len();
        if count > 0 {
            self.selected = if self.selected == 0 {
                count - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn cycle_filter(&mut self) {
        self.filter = match self.filter {
            FilterState::All => FilterState::Running,
            FilterState::Running => FilterState::Done,
            FilterState::Done => FilterState::Failed,
            FilterState::Failed => FilterState::All,
        };
        self.selected = 0;
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            SortField::Status => SortField::Name,
            SortField::Name => SortField::Started,
            SortField::Started => SortField::TurnCount,
            SortField::TurnCount => SortField::Status,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(id: &str, name: &str, status: AgentStatus) -> AgentSession {
        AgentSession {
            id: id.into(),
            name: name.into(),
            status,
            model: "test".into(),
            turn_count: 5,
            last_message: "done".into(),
            working_dir: "/tmp".into(),
            started_at: Instant::now(),
            task_subject: None,
        }
    }

    #[test]
    fn test_update_session() {
        let mut av = AgentView::new();
        av.update_session(make_session("1", "agent-a", AgentStatus::Running));
        assert_eq!(av.sessions.len(), 1);
        av.update_session(make_session("2", "agent-b", AgentStatus::Done));
        assert_eq!(av.sessions.len(), 2);
    }

    #[test]
    fn test_update_existing_session() {
        let mut av = AgentView::new();
        av.update_session(make_session("1", "agent-a", AgentStatus::Running));
        av.update_session(make_session("1", "agent-a", AgentStatus::Done));
        assert_eq!(av.sessions.len(), 1);
        assert_eq!(av.sessions[0].status, AgentStatus::Done);
    }

    #[test]
    fn test_remove_session() {
        let mut av = AgentView::new();
        av.update_session(make_session("1", "a", AgentStatus::Running));
        av.update_session(make_session("2", "b", AgentStatus::Done));
        av.remove_session("1");
        assert_eq!(av.sessions.len(), 1);
        assert_eq!(av.sessions[0].id, "2");
    }

    #[test]
    fn test_filter_by_status() {
        let mut av = AgentView::new();
        av.update_session(make_session("1", "a", AgentStatus::Running));
        av.update_session(make_session("2", "b", AgentStatus::Done));
        av.update_session(make_session("3", "c", AgentStatus::Failed));
        av.filter = FilterState::Running;
        assert_eq!(av.filtered_sessions().len(), 1);
        av.filter = FilterState::Done;
        assert_eq!(av.filtered_sessions().len(), 1);
        av.filter = FilterState::All;
        assert_eq!(av.filtered_sessions().len(), 3);
    }

    #[test]
    fn test_select_navigation() {
        let mut av = AgentView::new();
        av.update_session(make_session("1", "a", AgentStatus::Running));
        av.update_session(make_session("2", "b", AgentStatus::Running));
        assert_eq!(av.selected, 0);
        av.select_next();
        assert_eq!(av.selected, 1);
        av.select_next();
        assert_eq!(av.selected, 0); // wraps
        av.select_prev();
        assert_eq!(av.selected, 1); // wraps back
    }

    #[test]
    fn test_cycle_filter() {
        let mut av = AgentView::new();
        assert_eq!(av.filter, FilterState::All);
        av.cycle_filter();
        assert_eq!(av.filter, FilterState::Running);
        av.cycle_filter();
        assert_eq!(av.filter, FilterState::Done);
        av.cycle_filter();
        assert_eq!(av.filter, FilterState::Failed);
        av.cycle_filter();
        assert_eq!(av.filter, FilterState::All);
    }

    #[test]
    fn test_cycle_sort() {
        let mut av = AgentView::new();
        assert_eq!(av.sort_by, SortField::Status);
        av.cycle_sort();
        assert_eq!(av.sort_by, SortField::Name);
        av.cycle_sort();
        assert_eq!(av.sort_by, SortField::Started);
        av.cycle_sort();
        assert_eq!(av.sort_by, SortField::TurnCount);
        av.cycle_sort();
        assert_eq!(av.sort_by, SortField::Status);
    }
}
