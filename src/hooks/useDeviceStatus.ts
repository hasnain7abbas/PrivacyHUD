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
