use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use chrono::{DateTime, Utc};
use crate::storage::{Storage, Session};
use tauri::{AppHandle, Emitter};

pub struct SessionState {
    pub buffer: Vec<char>,
    pub cursor_pos: usize,
    pub created_at: DateTime<Utc>,
    pub last_input_at: Instant,
    pub is_recording: bool,
    pub session_timeout: u64,
    pub min_session_length: usize,
    pub capture_clipboard: bool,
}

pub struct SessionManager {
    pub state: Arc<Mutex<SessionState>>,
    app_handle: AppHandle,
}

impl SessionManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let storage = Storage::new(&app_handle);
        let timeout = storage.get_setting("session_timeout_seconds", "20").parse().unwrap_or(20);
        let min_len = storage.get_setting("min_session_length", "20").parse().unwrap_or(20);
        let capture_cb = storage.get_setting("enable_clipboard_capture", "true") == "true";
        let recording = storage.get_setting("enable_recording", "true") == "true";

        let state = Arc::new(Mutex::new(SessionState {
            buffer: Vec::new(),
            cursor_pos: 0,
            created_at: Utc::now(),
            last_input_at: Instant::now(),
            is_recording: recording,
            session_timeout: timeout,
            min_session_length: min_len,
            capture_clipboard: capture_cb,
        }));

        SessionManager { state, app_handle }
    }

    pub fn get_handle(&self) -> AppHandle {
        self.app_handle.clone()
    }

    pub fn handle_key(&self, text: Option<String>, special_key: Option<&str>) {
        let mut state = self.state.lock();
        if !state.is_recording {
            return;
        }

        let now = Instant::now();
        if !state.buffer.is_empty() && now.duration_since(state.last_input_at).as_secs() >= state.session_timeout {
            self.finalize_session_internal(&mut state);
        }

        state.last_input_at = now;
        if state.buffer.is_empty() {
            state.created_at = Utc::now();
        }

        if let Some(t) = text {
            for c in t.chars() {
                // Filter out non-printable control characters, except newline and tab
                if !c.is_control() || c == '\n' || c == '\t' || c == '\r' {
                    let pos = state.cursor_pos;
                    if pos <= state.buffer.len() {
                        state.buffer.insert(pos, c);
                        state.cursor_pos += 1;
                    }
                }
            }
        } else if let Some(sk) = special_key {
            match sk {
                "Backspace" => {
                    if state.cursor_pos > 0 && !state.buffer.is_empty() {
                        state.cursor_pos -= 1;
                        let pos = state.cursor_pos;
                        state.buffer.remove(pos);
                    }
                }
                "Delete" => {
                    let pos = state.cursor_pos;
                    if pos < state.buffer.len() {
                        state.buffer.remove(pos);
                    }
                }
                "Left" => {
                    if state.cursor_pos > 0 {
                        state.cursor_pos -= 1;
                    }
                }
                "Right" => {
                    if state.cursor_pos < state.buffer.len() {
                        state.cursor_pos += 1;
                    }
                }
                "Up" => {
                    let pos = state.cursor_pos;
                    let before = &state.buffer[..pos];
                    if let Some(last_nl) = before.iter().rposition(|&c| c == '\n') {
                        let offset = pos - last_nl - 1;
                        let far_before = &state.buffer[..last_nl];
                        let prev_line_start = far_before.iter().rposition(|&c| c == '\n').map(|x| x + 1).unwrap_or(0);
                        let prev_line_len = last_nl - prev_line_start;
                        state.cursor_pos = prev_line_start + std::cmp::min(offset, prev_line_len);
                    } else {
                        state.cursor_pos = 0;
                    }
                }
                "Down" => {
                    let pos = state.cursor_pos;
                    let after = &state.buffer[pos..];
                    if let Some(next_nl_rel) = after.iter().position(|&c| c == '\n') {
                        let next_nl = pos + next_nl_rel;
                        let before_this_line = &state.buffer[..pos];
                        let line_start = before_this_line.iter().rposition(|&c| c == '\n').map(|x| x + 1).unwrap_or(0);
                        let offset = pos - line_start;
                        
                        let after_next_nl = &state.buffer[(next_nl + 1)..];
                        let next_line_end_rel = after_next_nl.iter().position(|&c| c == '\n').unwrap_or(after_next_nl.len());
                        state.cursor_pos = next_nl + 1 + std::cmp::min(offset, next_line_end_rel);
                    } else {
                        state.cursor_pos = state.buffer.len();
                    }
                }
                "Home" => {
                    let pos = state.cursor_pos;
                    let before = &state.buffer[..pos];
                    state.cursor_pos = before.iter().rposition(|&c| c == '\n').map(|x| x + 1).unwrap_or(0);
                }
                "End" => {
                    let pos = state.cursor_pos;
                    let after = &state.buffer[pos..];
                    state.cursor_pos = pos + after.iter().position(|&c| c == '\n').unwrap_or(after.len());
                }
                "Enter" => {
                    let pos = state.cursor_pos;
                    state.buffer.insert(pos, '\n');
                    state.cursor_pos += 1;
                }
                _ => {}
            }
        }
    }

    pub fn handle_clipboard(&self, content: String) {
        let mut state = self.state.lock();
        if !state.is_recording || !state.capture_clipboard {
            return;
        }

        let now = Instant::now();
        if !state.buffer.is_empty() && now.duration_since(state.last_input_at).as_secs() >= state.session_timeout {
            self.finalize_session_internal(&mut state);
        }

        state.last_input_at = now;
        if state.buffer.is_empty() {
            state.created_at = Utc::now();
        }

        for c in content.chars() {
            if !c.is_control() || c == '\n' || c == '\t' || c == '\r' {
                let pos = state.cursor_pos;
                if pos <= state.buffer.len() {
                    state.buffer.insert(pos, c);
                    state.cursor_pos += 1;
                }
            }
        }
    }

    pub fn finalize_session(&self) {
        let mut state = self.state.lock();
        self.finalize_session_internal(&mut state);
    }

    fn finalize_session_internal(&self, state: &mut SessionState) {
        if state.buffer.len() >= state.min_session_length {
            let content: String = state.buffer.iter().collect();
            if content.trim().len() >= state.min_session_length {
                let storage = Storage::new(&self.app_handle);
                let session = Session {
                    id: None,
                    created_at: state.created_at,
                    ended_at: Utc::now(),
                    content: content.clone(),
                    char_count: state.buffer.len() as i32,
                };
                let _ = storage.save_session(&session);
                // Notify frontend if needed (via events)
                let _ = self.app_handle.emit("session-saved", ());
            }
        }
        state.buffer.clear();
        state.cursor_pos = 0;
    }

    pub fn start_timeout_monitor(manager: Arc<SessionManager>) {
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(5));
                let mut state = manager.state.lock();
                if !state.buffer.is_empty() && Instant::now().duration_since(state.last_input_at).as_secs() >= state.session_timeout {
                    manager.finalize_session_internal(&mut state);
                }
            }
        });
    }
}
