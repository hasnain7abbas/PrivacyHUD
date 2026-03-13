use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};

pub fn setup_hotkeys() -> GlobalHotKeyManager {
    let manager = GlobalHotKeyManager::new().expect("Failed to init hotkey manager");

    // Ctrl+Shift+M -> Toggle mic mute
    let mic_hotkey = HotKey::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT),
        Code::KeyM,
    );

    // Ctrl+Shift+V -> Toggle camera
    let cam_hotkey = HotKey::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT),
        Code::KeyV,
    );

    manager.register(mic_hotkey).expect("Failed to register mic hotkey");
    manager.register(cam_hotkey).expect("Failed to register cam hotkey");

    manager
}

pub fn toggle_mic_mute() {
    // TODO: Use IAudioEndpointVolume::SetMute on the default capture device
}

pub fn toggle_camera() {
    // TODO: Disable/enable camera device node via SetupDi* APIs
}
