use std::collections::{HashMap, VecDeque};

#[derive(Debug, Default)]
pub struct SessionStore {
    sessions: HashMap<String, Session>,
    max_history: usize,
}

#[derive(Debug, Default)]
pub struct Session {
    prompts: VecDeque<String>,
    last_command: Option<String>,
}

impl SessionStore {
    pub fn new(max_history: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            max_history,
        }
    }

    pub fn record_input(&mut self, session_id: &str, prompt: &str) {
        let session = self.sessions.entry(session_id.to_string()).or_default();
        session.prompts.push_back(prompt.to_string());
        while session.prompts.len() > self.max_history {
            session.prompts.pop_front();
        }
    }

    pub fn record_output(&mut self, session_id: &str, command: &str) {
        let session = self.sessions.entry(session_id.to_string()).or_default();
        session.last_command = Some(command.to_string());
    }

    pub fn snapshot(&self, session_id: &str) -> SessionSnapshot {
        let session = self.sessions.get(session_id);
        SessionSnapshot {
            history: session
                .map(|s| s.prompts.iter().cloned().collect())
                .unwrap_or_default(),
            last_command: session.and_then(|s| s.last_command.clone()),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SessionSnapshot {
    pub history: Vec<String>,
    pub last_command: Option<String>,
}
