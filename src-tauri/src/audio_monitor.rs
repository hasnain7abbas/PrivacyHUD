use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

pub fn is_mic_active() -> Result<bool, Box<dyn std::error::Error>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?;

        let session_mgr: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        let session_enum = session_mgr.GetSessionEnumerator()?;

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
