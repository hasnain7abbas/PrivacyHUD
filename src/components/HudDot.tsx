import { useDeviceStatus } from "../hooks/useDeviceStatus";
import "../styles/hud.css";

export function HudDot() {
  const { mic_active, cam_active } = useDeviceStatus();

  const active = mic_active || cam_active;
  const color = active ? (mic_active ? "red" : "yellow") : "green";

  const tooltip = mic_active
    ? `MIC LIVE${cam_active ? " + CAM LIVE" : ""}`
    : cam_active
      ? "CAM LIVE"
      : "All clear";

  return (
    <div className="hud-wrapper" data-tauri-drag-region>
      <div
        className={`hud-dot hud-dot--${color}${active ? " hud-dot--active" : ""}`}
        title={tooltip}
        data-tauri-drag-region
      />
    </div>
  );
}
