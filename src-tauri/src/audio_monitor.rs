use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Registry::*;
use windows::core::{HSTRING, PWSTR};

const MIC_CONSENT_KEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";

pub fn is_mic_active() -> Result<bool, Box<dyn std::error::Error>> {
    // Method 1: Check Windows consent store registry
    if check_mic_registry() {
        return Ok(true);
    }

    // Method 2: Check audio capture sessions
    if check_audio_sessions() {
        return Ok(true);
    }

    Ok(false)
}

fn check_mic_registry() -> bool {
    let base_key = HKEY_CURRENT_USER;
    for sub in &["", "NonPackaged"] {
        let path = if sub.is_empty() {
            MIC_CONSENT_KEY.to_string()
        } else {
            format!("{}\\{}", MIC_CONSENT_KEY, sub)
        };
        if check_registry_subkeys(base_key, &path).unwrap_or(false) {
            return true;
        }
    }
    false
}

fn check_registry_subkeys(base: HKEY, path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(base, &HSTRING::from(path), 0, KEY_READ, &mut key).is_err() {
            return Ok(false);
        }

        let mut index = 0u32;
        let mut name_buf = [0u16; 512];

        loop {
            let mut name_len = name_buf.len() as u32;
            let status = RegEnumKeyExW(
                key,
                index,
                PWSTR(name_buf.as_mut_ptr()),
                &mut name_len,
                None,
                PWSTR::null(),
                None,
                None,
            );

            if status.is_err() {
                break;
            }

            let subkey_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            let subkey_path = format!("{}\\{}", path, subkey_name);

            let mut sub_key = HKEY::default();
            if RegOpenKeyExW(base, &HSTRING::from(&subkey_path), 0, KEY_READ, &mut sub_key).is_ok()
            {
                let active = is_entry_active(sub_key);
                let _ = RegCloseKey(sub_key);
                if active {
                    let _ = RegCloseKey(key);
                    return Ok(true);
                }
            }

            index += 1;
        }

        let _ = RegCloseKey(key);
        Ok(false)
    }
}

/// Check if a ConsentStore entry indicates active device usage.
/// Active = LastUsedTimeStop is 0 AND LastUsedTimeStart > 0.
unsafe fn is_entry_active(key: HKEY) -> bool {
    let mut stop_bytes = [0xFFu8; 8]; // default to non-zero so failed reads = inactive
    let mut data_size = 8u32;

    let stop_ok = RegQueryValueExW(
        key,
        &HSTRING::from("LastUsedTimeStop"),
        None,
        None,
        Some(stop_bytes.as_mut_ptr()),
        Some(&mut data_size),
    );

    if stop_ok.is_err() {
        return false;
    }

    let stop_value = u64::from_le_bytes(stop_bytes);
    if stop_value != 0 {
        return false;
    }

    // Stop is 0 — check that start > 0 to confirm genuine usage
    let mut start_bytes = [0u8; 8];
    let mut start_size = 8u32;

    let start_ok = RegQueryValueExW(
        key,
        &HSTRING::from("LastUsedTimeStart"),
        None,
        None,
        Some(start_bytes.as_mut_ptr()),
        Some(&mut start_size),
    );

    if start_ok.is_err() {
        return false;
    }

    let start_value = u64::from_le_bytes(start_bytes);
    start_value > 0
}

fn check_audio_sessions() -> bool {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                Ok(e) => e,
                Err(_) => return false,
            };

        let device = match enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) {
            Ok(d) => d,
            Err(_) => return false,
        };

        let session_mgr: IAudioSessionManager2 = match device.Activate(CLSCTX_ALL, None) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let session_enum = match session_mgr.GetSessionEnumerator() {
            Ok(e) => e,
            Err(_) => return false,
        };

        let count = match session_enum.GetCount() {
            Ok(c) => c,
            Err(_) => return false,
        };

        for i in 0..count {
            if let Ok(session) = session_enum.GetSession(i) {
                if let Ok(state) = session.GetState() {
                    if state == AudioSessionStateActive {
                        return true;
                    }
                }
            }
        }

        false
    }
}
