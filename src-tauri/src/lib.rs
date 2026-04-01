mod storage;
mod session_manager;
mod keyboard;
mod commands;

use std::sync::Arc;
// use std::sync::atomic; // removed
use crate::session_manager::SessionManager;
use crate::keyboard::KeyboardHook;
use tauri_plugin_autostart::MacosLauncher;
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            println!("App is setting up...");
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, Modifiers, Code};
            let app_handle = app.handle();
            let ctrl_shift_space = Shortcut::new(
                Some(Modifiers::CONTROL | Modifiers::SHIFT),
                Code::Space,
            );
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, _event| {
                        if shortcut == &ctrl_shift_space {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(),
            )?;
            app.global_shortcut().register(ctrl_shift_space)?;

            let manager = Arc::new(SessionManager::new(app_handle.clone()));
            app.manage(manager.clone());

            // Start session timeout monitor
            SessionManager::start_timeout_monitor(manager.clone());

            // Start keyboard hook
            let hook = KeyboardHook::new(manager.clone());
            hook.start();

            // Tray setup
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let open_i = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
            let pause_i = MenuItem::with_id(app, "pause", "Pause Recording", true, None::<&str>)?;
            let resume_i = MenuItem::with_id(app, "resume", "Resume Recording", true, None::<&str>)?;
            
            let tray_menu = Menu::with_items(app, &[
                &open_i,
                &PredefinedMenuItem::separator(app)?,
                &pause_i,
                &resume_i,
                &PredefinedMenuItem::separator(app)?,
                &quit_i,
            ])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "quit" => app.exit(0),
                        "open" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "pause" => {
                            let manager = app.state::<Arc<SessionManager>>();
                            let mut state = manager.state.lock();
                            state.is_recording = false;
                        }
                        "resume" => {
                            let manager = app.state::<Arc<SessionManager>>();
                            let mut state = manager.state.lock();
                            state.is_recording = true;
                        }
                        _ => (),
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_sessions,
            commands::delete_sessions,
            commands::save_setting,
            commands::get_setting,
            commands::toggle_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
