import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type SyncStatus = "idle" | "syncing" | "error" | "offline";

const STATUS_COLORS: Record<SyncStatus, string> = {
  idle: "#4caf50",
  syncing: "#ff9800",
  error: "#f44336",
  offline: "#9e9e9e",
};

const STATUS_LABELS: Record<SyncStatus, string> = {
  idle: "Synced",
  syncing: "Syncing...",
  error: "Error",
  offline: "Offline",
};

export default function Status() {
  const [status, setStatus] = useState<SyncStatus>("idle");
  const [lastSync, setLastSync] = useState<string | null>(null);
  const [lastDevice, setLastDevice] = useState<string | null>(null);
  const [message, setMessage] = useState<string>("");
  const [busy, setBusy] = useState(false);

  async function runCommand(command: string) {
    if (busy) return;
    setBusy(true);
    setStatus("syncing");
    setMessage("");
    try {
      const result = await invoke<string>(command);
      setStatus("idle");
      setLastSync(new Date().toLocaleString());
      // Parse device name from result if present (e.g. "Pulled { from_device: \"my-pc\" }")
      const deviceMatch = result.match(/from_device:\s*"([^"]+)"/);
      if (deviceMatch) {
        setLastDevice(deviceMatch[1]);
      }
      setMessage(result);
    } catch (err) {
      setStatus("error");
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page">
      <h2>Sync Status</h2>

      <div className="status-card">
        <div className="status-indicator-row">
          <span
            className="status-dot"
            style={{ backgroundColor: STATUS_COLORS[status] }}
          />
          <span className="status-label">{STATUS_LABELS[status]}</span>
        </div>

        {lastSync && (
          <div className="status-detail">
            <span className="detail-label">Last sync:</span>
            <span>{lastSync}</span>
          </div>
        )}
        {lastDevice && (
          <div className="status-detail">
            <span className="detail-label">Device:</span>
            <span>{lastDevice}</span>
          </div>
        )}
      </div>

      <div className="button-group">
        <button
          className="btn btn-primary"
          onClick={() => runCommand("sync_now")}
          disabled={busy}
        >
          Sync Now
        </button>
        <button
          className="btn btn-secondary"
          onClick={() => runCommand("push_now")}
          disabled={busy}
        >
          Push
        </button>
        <button
          className="btn btn-secondary"
          onClick={() => runCommand("pull_now")}
          disabled={busy}
        >
          Pull
        </button>
      </div>

      {message && (
        <div className={`message ${status === "error" ? "message-error" : "message-success"}`}>
          {message}
        </div>
      )}
    </div>
  );
}
