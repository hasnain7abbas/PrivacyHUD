# 🛡️ PrivacyHUD: Global Mic & Camera Privacy Monitor

## 📖 Overview

**PrivacyHUD** is an ultra-lightweight, system-level utility designed to give users absolute peace of mind regarding their microphone and webcam statuses. Instead of relying on individual app settings (like Zoom, Teams, or Discord), PrivacyHUD provides a single, unobtrusive visual indicator showing the *actual* hardware state of your input devices.

If your mic is hot, you know. If your camera is active, you know.

## ✨ Key Features

* **System-Level Monitoring:** Reads directly from the OS API to determine if any process is pulling data from the microphone or webcam.
* **Unobtrusive Visual HUD:** A minimal floating dot (or taskbar icon) that stays out of your way.
    * 🟢 Green: Hardware inactive (Safe)
    * 🔴 Red: Hardware active (Live)
* **Global Kill-Switch Hotkeys:** Instantly sever the microphone or camera connection at the OS level using customizable keyboard shortcuts (e.g., `Ctrl + Shift + M`), regardless of which app is in focus.
* **Zero-Bloat Footprint:** Designed to run on less than 15MB of RAM with negligible CPU usage.
* **App Agnostic:** Works flawlessly across all communication platforms.

---

## 🛑 The Problem vs. 💡 The Solution

| The Problem | The PrivacyHUD Solution |
| :--- | :--- |
| You mute yourself in a Zoom meeting, but aren't sure if your browser is still listening. | Reads the raw hardware state. If the OS says the mic is active, the HUD turns red. One source of truth. |
| You have to frantically tab through 5 windows to find where the unmuted audio is coming from. | Press your global hotkey to kill the mic instantly, without needing to find the offending app. |
| Heavy, clunky manufacturer bloatware is required to manage hardware settings. | A tiny, focused utility that does exactly one thing perfectly, taking up almost zero system resources. |

---

## 🛠️ Tech Stack (Tauri + Rust)

| Layer | Technology | Role |
| :--- | :--- | :--- |
| Backend | **Rust** | System-level device polling, hotkey hooks, muting logic |
| App Shell | **Tauri 2.0** | Lightweight window management, IPC, system tray, bundling |
| Frontend | **React + TypeScript** | Minimal HUD rendering and settings UI |
| Styling | **CSS (vanilla)** | Frameless transparent window, dot indicator |

---

## 🏗️ Project Structure

```
privacyhud/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri entry point, app builder
│   │   ├── lib.rs               # Module declarations & Tauri command exports
│   │   ├── audio_monitor.rs     # WASAPI / CoreAudio mic state polling
│   │   ├── video_monitor.rs     # Registry / Media Foundation webcam detection
│   │   ├── killswitch.rs        # Global hotkey registration & device muting
│   │   ├── tray.rs              # System tray icon + context menu
│   │   └── state.rs             # Shared DeviceStatus state management
│   ├── icons/                   # App icons for bundling
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.tsx                  # Root component
│   ├── components/
│   │   ├── HudDot.tsx           # Floating colored indicator dot
│   │   └── SettingsPanel.tsx    # Hotkey config & preferences UI
│   ├── hooks/
│   │   └── useDeviceStatus.ts   # Tauri event listener for device state
│   └── styles/
│       └── hud.css              # Frameless transparent window styles
├── package.json
├── tsconfig.json
└── README.md
```

---

## 📦 Core Rust Dependencies

```toml
# src-tauri/Cargo.toml

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
global-hotkey = "0.6"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Media_Audio",
    "Win32_System_Com",
    "Win32_Devices_FunctionDiscovery",
    "Win32_Security",
    "Win32_System_Registry",
]}
```

---

## ⚙️ Implementation Details

### 1. Device Status Model

```rust
// state.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HudColor {
    Green,  // All devices inactive
    Red,    // Mic and/or camera active
    Yellow, // Camera only (optional granularity)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub mic_active: bool,
    pub cam_active: bool,
    pub active_processes: Vec<String>, // Which apps are using the devices
}

impl DeviceStatus {
    pub fn hud_color(&self) -> HudColor {
        match (self.mic_active, self.cam_active) {
            (false, false) => HudColor::Green,
            (false, true)  => HudColor::Yellow,
            _              => HudColor::Red,
        }
    }
}
```

### 2. Microphone Monitoring (Windows — WASAPI)

Uses the `IAudioSessionManager2` COM interface to enumerate active capture sessions. Polled on a background thread every ~500ms.

```rust
// audio_monitor.rs — simplified pseudocode
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

pub fn is_mic_active() -> Result<bool, Box<dyn std::error::Error>> {
    unsafe {
        // 1. Initialize COM
        CoInitializeEx(None, COINIT_MULTITHREADED)?;

        // 2. Get default capture (microphone) device
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?;

        // 3. Activate session manager
        let session_mgr: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        let session_enum = session_mgr.GetSessionEnumerator()?;

        // 4. Check each session for active state
        let count = session_enum.GetCount()?;
        for i in 0..count {
            let session = session_enum.GetSession(i)?;
            if session.GetState()? == AudioSessionStateActive {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
```

### 3. Webcam Monitoring (Windows — Registry)

Windows tracks camera usage in the Capability Access Manager registry. If a `LastUsedTimeStop` value is `0`, the camera is currently active.

```rust
// video_monitor.rs — simplified pseudocode
use windows::Win32::System::Registry::*;

const CAM_CONSENT_KEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\webcam";

pub fn is_camera_active() -> Result<bool, Box<dyn std::error::Error>> {
    // Walk child keys under both "NonPackaged" and packaged app entries
    // For each app subkey:
    //   Read "LastUsedTimeStart" and "LastUsedTimeStop"
    //   If LastUsedTimeStop == 0 && LastUsedTimeStart > 0 → camera is active
    //   Collect process name from the key path

    Ok(false) // placeholder
}
```

### 4. Global Kill-Switch (Hotkeys)

```rust
// killswitch.rs
use global_hotkey::{GlobalHotKeyManager, GlobalHotKeyEvent, hotkey::{HotKey, Modifiers, Code}};

pub fn setup_hotkeys() -> GlobalHotKeyManager {
    let manager = GlobalHotKeyManager::new().expect("Failed to init hotkey manager");

    // Ctrl+Shift+M → Toggle mic mute
    let mic_hotkey = HotKey::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT),
        Code::KeyM,
    );

    // Ctrl+Shift+V → Toggle camera
    let cam_hotkey = HotKey::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT),
        Code::KeyV,
    );

    manager.register(mic_hotkey).expect("Failed to register mic hotkey");
    manager.register(cam_hotkey).expect("Failed to register cam hotkey");

    manager
}

pub fn toggle_mic_mute() {
    // Use IAudioEndpointVolume::SetMute on the default capture device
    // Flip the current mute state
}

pub fn toggle_camera() {
    // Disable/enable camera device node via SetupDi* APIs
    // or DeviceIoControl — requires elevated privileges
}
```

### 5. Tauri App Entry Point

```rust
// main.rs
mod audio_monitor;
mod video_monitor;
mod killswitch;
mod state;
mod tray;

use state::DeviceStatus;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    let device_state = Arc::new(Mutex::new(DeviceStatus {
        mic_active: false,
        cam_active: false,
        active_processes: vec![],
    }));

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let state = device_state.clone();

            // Spawn background polling loop
            std::thread::spawn(move || {
                loop {
                    let mic = audio_monitor::is_mic_active().unwrap_or(false);
                    let cam = video_monitor::is_camera_active().unwrap_or(false);

                    let mut s = state.lock().unwrap();
                    s.mic_active = mic;
                    s.cam_active = cam;

                    // Emit event to frontend
                    let _ = handle.emit("device-status", s.clone());

                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            });

            // Register global hotkeys
            killswitch::setup_hotkeys();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running PrivacyHUD");
}
```

### 6. Tauri Window Config

```json
{
  "app": {
    "windows": [
      {
        "label": "hud",
        "title": "PrivacyHUD",
        "width": 48,
        "height": 48,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "resizable": false,
        "skipTaskbar": true,
        "x": 20,
        "y": 20
      }
    ],
    "trayIcon": {
      "iconPath": "icons/tray-green.png",
      "tooltip": "PrivacyHUD — All devices inactive"
    }
  }
}
```

---

## 🖥️ Frontend (React + TypeScript)

### Device Status Hook

```typescript
// hooks/useDeviceStatus.ts
import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

interface DeviceStatus {
  mic_active: boolean;
  cam_active: boolean;
  active_processes: string[];
}

export function useDeviceStatus() {
  const [status, setStatus] = useState<DeviceStatus>({
    mic_active: false,
    cam_active: false,
    active_processes: [],
  });

  useEffect(() => {
    const unlisten = listen<DeviceStatus>("device-status", (event) => {
      setStatus(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return status;
}
```

### HUD Dot Component

```tsx
// components/HudDot.tsx
import { useDeviceStatus } from "../hooks/useDeviceStatus";
import "../styles/hud.css";

export function HudDot() {
  const { mic_active, cam_active } = useDeviceStatus();

  const color = mic_active || cam_active
    ? mic_active ? "red" : "yellow"
    : "green";

  const tooltip = mic_active
    ? `MIC LIVE${cam_active ? " + CAM LIVE" : ""}`
    : cam_active
      ? "CAM LIVE"
      : "All clear";

  return (
    <div
      className={`hud-dot hud-dot--${color}`}
      title={tooltip}
      data-tauri-drag-region
    />
  );
}
```

### HUD Styles

```css
/* styles/hud.css */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background: transparent;
  overflow: hidden;
}

.hud-dot {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  margin: 8px;
  cursor: grab;
  transition: background-color 0.3s ease, box-shadow 0.3s ease;
}

.hud-dot--green {
  background-color: #22c55e;
  box-shadow: 0 0 8px rgba(34, 197, 94, 0.6);
}

.hud-dot--red {
  background-color: #ef4444;
  box-shadow: 0 0 12px rgba(239, 68, 68, 0.8);
  animation: pulse-red 1.5s infinite;
}

.hud-dot--yellow {
  background-color: #eab308;
  box-shadow: 0 0 10px rgba(234, 179, 8, 0.7);
  animation: pulse-yellow 2s infinite;
}

@keyframes pulse-red {
  0%, 100% { box-shadow: 0 0 12px rgba(239, 68, 68, 0.8); }
  50%      { box-shadow: 0 0 20px rgba(239, 68, 68, 1.0); }
}

@keyframes pulse-yellow {
  0%, 100% { box-shadow: 0 0 10px rgba(234, 179, 8, 0.7); }
  50%      { box-shadow: 0 0 16px rgba(234, 179, 8, 0.9); }
}
```

---

## 📊 Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│  Rust Backend (src-tauri)                                   │
│                                                             │
│  audio_monitor ──┐                                          │
│  (WASAPI poll)   ├──► state.rs ──► Tauri Event Emitter ────┤──► Frontend
│  video_monitor ──┘   (DeviceStatus)                         │
│  (Registry poll)         ▲                                  │
│                          │                                  │
│  killswitch ─────────────┘                                  │
│  (global hotkey toggles mute → updates state)               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  React Frontend (src/)                                      │
│                                                             │
│  useDeviceStatus() ──► listens to "device-status" event     │
│       │                                                     │
│       ▼                                                     │
│  <HudDot />                                                 │
│    🟢  mic=off, cam=off                                     │
│    🔴  mic=on                                               │
│    🟡  cam=on, mic=off                                      │
└─────────────────────────────────────────────────────────────┘
```

---

## 🚀 Roadmap

### Version 1.0

1. **Phase 1: Audio Hook.** Poll the OS to detect if the default microphone is actively capturing audio.
2. **Phase 2: The HUD.** Create the floating, always-on-top frameless window that changes color.
3. **Phase 3: The Kill-Switch.** Global keyboard hook to mute the system default recording device.
4. **Phase 4: Video Hook.** Registry/API polling to detect active webcam usage.

### Post-1.0

* **Process Attribution** — Show which app is using the mic/camera on hover tooltip.
* **Activity Log** — Timestamped history of device activations.
* **macOS Support** — `AVFoundation` + `IOKit` monitoring.
* **Linux Support** — PulseAudio/PipeWire via `libpulse` bindings.
* **Auto-Start** — Launch at system boot via native autostart.
* **Notification Alerts** — Optional toast notification when a device is activated.

---

## 🏃 Quick Start

```bash
# Prerequisites: Rust (stable), Node.js 18+, Tauri CLI v2
npm install -g @tauri-apps/cli

# Clone and install
git clone https://github.com/youruser/privacyhud.git
cd privacyhud
npm install

# Development
cargo tauri dev

# Production build (outputs .msi / .exe installer)
cargo tauri build
```

---

## 🤝 Contributing

*(Placeholder for open-source contribution guidelines)*

## 📄 License

MIT License — Free and open for anyone to use, modify, and distribute.
