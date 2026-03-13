mod audio_monitor;
mod killswitch;
mod state;
mod tray;
mod video_monitor;

use state::DeviceStatus;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let device_state = Arc::new(Mutex::new(DeviceStatus {
        mic_active: false,
        cam_active: false,
        active_processes: vec![],
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![quit_app])
        .setup(move |app| {
            let handle = app.handle().clone();
            let state = device_state.clone();

            // Setup system tray with quit option
            if let Err(e) = tray::setup_tray(app) {
                eprintln!("Failed to setup tray: {}", e);
            }

            // Spawn background polling loop
            std::thread::spawn(move || loop {
                let mic = audio_monitor::is_mic_active().unwrap_or(false);
                let cam = video_monitor::is_camera_active().unwrap_or(false);

                let mut s = state.lock().unwrap();
                s.mic_active = mic;
                s.cam_active = cam;

                let _ = handle.emit("device-status", s.clone());

                std::thread::sleep(std::time::Duration::from_millis(500));
            });

            killswitch::setup_hotkeys();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running PrivacyHUD");
}
