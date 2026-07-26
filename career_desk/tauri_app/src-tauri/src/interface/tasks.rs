use crate::{error::ErrorBody, infra::llm::CancellationToken};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
const MAX_ACTIVE_TASKS: usize = 4;
const TERMINAL_RETENTION: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
    pub task_id: String,
    pub operation: String,
    pub state: TaskState,
    pub progress: Option<f32>,
    pub result: Option<serde_json::Value>,
    pub error: Option<ErrorBody>,
    pub events: Vec<crate::domain::llm::StreamEvent>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Started,
    Progress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone)]
struct TaskEntry {
    event: TaskEvent,
    cancel: CancellationToken,
    updated_at: Instant,
}

#[derive(Default)]
pub struct TaskRegistry {
    entries: Mutex<HashMap<String, TaskEntry>>,
}

impl TaskRegistry {
    fn cleanup(entries: &mut HashMap<String, TaskEntry>) {
        entries.retain(|_, entry| {
            !matches!(
                entry.event.state,
                TaskState::Completed | TaskState::Failed | TaskState::Cancelled
            ) || entry.updated_at.elapsed() < TERMINAL_RETENTION
        })
    }
    pub fn register(
        &self,
        id: String,
        operation: String,
    ) -> Result<(TaskEvent, CancellationToken), &'static str> {
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        Self::cleanup(&mut entries);
        if entries
            .values()
            .filter(|entry| matches!(entry.event.state, TaskState::Started | TaskState::Progress))
            .count()
            >= MAX_ACTIVE_TASKS
        {
            return Err("at most four background tasks may run concurrently");
        }
        let cancel = CancellationToken::default();
        let event = TaskEvent {
            task_id: id.clone(),
            operation,
            state: TaskState::Started,
            progress: Some(0.0),
            result: None,
            error: None,
            events: vec![],
        };
        entries.insert(
            id,
            TaskEntry {
                event: event.clone(),
                cancel: cancel.clone(),
                updated_at: Instant::now(),
            },
        );
        Ok((event, cancel))
    }
    pub fn get(&self, id: &str) -> Option<TaskEvent> {
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)
            .map(|x| x.event.clone())
    }
    pub fn update(&self, event: TaskEvent) {
        if let Some(entry) = self
            .entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_mut(&event.task_id)
        {
            entry.event = event;
            entry.updated_at = Instant::now();
        }
    }
    pub fn append_stream_event(
        &self,
        id: &str,
        event: crate::domain::llm::StreamEvent,
    ) -> Option<TaskEvent> {
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let entry = entries.get_mut(id)?;
        if matches!(
            entry.event.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        ) {
            return Some(entry.event.clone());
        }
        entry.event.events.push(event.clone());
        entry.event.state = TaskState::Progress;
        entry.event.progress = match event {
            crate::domain::llm::StreamEvent::Started { .. } => Some(5.0),
            crate::domain::llm::StreamEvent::Retrying { .. } => Some(5.0),
            _ => None,
        };
        entry.updated_at = Instant::now();
        Some(entry.event.clone())
    }
    pub fn cancel(&self, id: &str) -> Option<TaskEvent> {
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let entry = entries.get_mut(id)?;
        if matches!(
            entry.event.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        ) {
            return Some(entry.event.clone());
        }
        entry.cancel.cancel();
        entry.event.state = TaskState::Cancelled;
        entry.event.progress = None;
        entry.updated_at = Instant::now();
        Some(entry.event.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_lifecycle_is_queryable_and_cancelled() {
        let r = TaskRegistry::default();
        let (e, _) = r.register("a".into(), "pdf".into()).unwrap();
        assert_eq!(e.state, TaskState::Started);
        assert_eq!(r.cancel("a").unwrap().state, TaskState::Cancelled);
        assert_eq!(r.get("a").unwrap().state, TaskState::Cancelled);
    }
    #[test]
    fn limits_active_workers() {
        let r = TaskRegistry::default();
        for n in 0..4 {
            r.register(n.to_string(), "x".into()).unwrap();
        }
        assert!(r.register("5".into(), "x".into()).is_err());
        r.cancel("0");
        assert!(r.register("6".into(), "x".into()).is_ok());
    }
    #[test]
    fn expired_terminal_tasks_are_removed_on_registration() {
        let r = TaskRegistry::default();
        r.register("old".into(), "x".into()).unwrap();
        {
            let mut entries = r.entries.lock().unwrap();
            let entry = entries.get_mut("old").unwrap();
            entry.event.state = TaskState::Completed;
            entry.updated_at = Instant::now() - TERMINAL_RETENTION - Duration::from_secs(1);
        }
        r.register("new".into(), "x".into()).unwrap();
        assert!(r.get("old").is_none());
        assert!(r.get("new").is_some());
    }
    #[test]
    fn stream_events_are_queryable_and_indeterminate_progress_is_honest() {
        let r = TaskRegistry::default();
        r.register("s".into(), "llm".into()).unwrap();
        r.append_stream_event(
            "s",
            crate::domain::llm::StreamEvent::Started {
                provider: "p".into(),
                model: "m".into(),
                attempt: 1,
            },
        );
        let delta = r
            .append_stream_event(
                "s",
                crate::domain::llm::StreamEvent::Delta { text: "tok".into() },
            )
            .unwrap();
        assert_eq!(delta.state, TaskState::Progress);
        assert_eq!(delta.progress, None);
        assert_eq!(r.get("s").unwrap().events.len(), 2);
        assert_eq!(r.cancel("s").unwrap().state, TaskState::Cancelled)
    }
    #[test]
    fn cancel_is_idempotent_and_late_tokens_cannot_resurrect_terminal_task() {
        let r = TaskRegistry::default();
        r.register("race".into(), "llm".into()).unwrap();
        let first = r.cancel("race").unwrap();
        let second = r.cancel("race").unwrap();
        assert_eq!(first.state, TaskState::Cancelled);
        assert_eq!(second.state, TaskState::Cancelled);
        let late = r
            .append_stream_event(
                "race",
                crate::domain::llm::StreamEvent::Delta {
                    text: "late".into(),
                },
            )
            .unwrap();
        assert_eq!(late.state, TaskState::Cancelled);
        assert!(late.events.is_empty())
    }
}
