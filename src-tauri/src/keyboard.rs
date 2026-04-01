use rdev::{listen, EventType, Key};
use std::sync::Arc;
use crate::session_manager::SessionManager;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct KeyboardHook {
    manager: Arc<SessionManager>,
    ctrl_pressed: Arc<AtomicBool>,
}

impl KeyboardHook {
    pub fn new(manager: Arc<SessionManager>) -> Self {
        KeyboardHook {
            manager,
            ctrl_pressed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        let manager = Arc::clone(&self.manager);
        let ctrl_pressed = Arc::clone(&self.ctrl_pressed);

        std::thread::spawn(move || {
            if let Err(error) = listen(move |event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        match key {
                            Key::ControlLeft | Key::ControlRight => {
                                ctrl_pressed.store(true, Ordering::SeqCst);
                            }
                            Key::KeyV if ctrl_pressed.load(Ordering::SeqCst) => {
                                // Ctrl + V detected
                                let manager_clone = Arc::clone(&manager);
                                let app_handle = manager_clone.get_handle();
                                tauri::async_runtime::spawn(async move {
                                    use tauri_plugin_clipboard_manager::ClipboardExt;
                                    if let Ok(text) = app_handle.clipboard().read_text() {
                                        manager_clone.handle_clipboard(text);
                                    }
                                });
                            }
                            Key::Backspace => manager.handle_key(None, Some("Backspace")),
                            Key::Delete => manager.handle_key(None, Some("Delete")),
                            Key::LeftArrow => manager.handle_key(None, Some("Left")),
                            Key::RightArrow => manager.handle_key(None, Some("Right")),
                            Key::UpArrow => manager.handle_key(None, Some("Up")),
                            Key::DownArrow => manager.handle_key(None, Some("Down")),
                            Key::Home => manager.handle_key(None, Some("Home")),
                            Key::End => manager.handle_key(None, Some("End")),
                            Key::Return => manager.handle_key(None, Some("Enter")),
                            _ => {
                                // Extract character from event
                                if let Some(text) = event.name {
                                    manager.handle_key(Some(text), None);
                                }
                            }
                        }
                    }
                    EventType::KeyRelease(key) => {
                        match key {
                            Key::ControlLeft | Key::ControlRight => {
                                ctrl_pressed.store(false, Ordering::SeqCst);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }) {
                println!("Error: {:?}", error);
            }
        });
    }
}
