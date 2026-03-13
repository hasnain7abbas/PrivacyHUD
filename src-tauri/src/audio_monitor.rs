use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Registry::*;
use windows::core::{HSTRING, PWSTR};

const MIC_CONSENT_KEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";

pub fn is_mic_active() -> Result<bool, Box<dyn std::error::Error>> {
    // Primary: Check Windows consent store registry (most reliable on Windows 10/11)
    if check_mic_registry() {
        return Ok(true);
    }

    // Fallback: Check audio capture sessions
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
        let result = RegOpenKeyExW(base, &HSTRING::from(path), 0, KEY_READ, &mut key);
        if result.is_err() {
            return Ok(false);
        }

        let mut index = 0u32;
        let mut name_buf = [0u16; 256];

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
                let mut stop_value: u64 = 1;
                let mut data_size = std::mem::size_of::<u64>() as u32;

                let read_result = RegQueryValueExW(
                    sub_key,
                    &HSTRING::from("LastUsedTimeStop"),
                    None,
                    None,
                    Some((&mut stop_value as *mut u64).cast()),
                    Some(&mut data_size),
                );

                if read_result.is_ok() && stop_value == 0 {
                    let mut start_value: u64 = 0;
                    let mut start_size = std::mem::size_of::<u64>() as u32;

                    let start_result = RegQueryValueExW(
                        sub_key,
                        &HSTRING::from("LastUsedTimeStart"),
                        None,
                        None,
                        Some((&mut start_value as *mut u64).cast()),
                        Some(&mut start_size),
                    );

                    if start_result.is_ok() && start_value > 0 {
                        let _ = RegCloseKey(sub_key);
                        let _ = RegCloseKey(key);
                        return Ok(true);
                    }
                }

                let _ = RegCloseKey(sub_key);
            }

            index += 1;
        }

        let _ = RegCloseKey(key);
        Ok(false)
    }
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
