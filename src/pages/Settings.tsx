import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AppConfig {
  steelseries_db_path: string;
  backup_dir: string;
  max_backups: number;
  debounce_secs: number;
  provider: ProviderConfig;
  device_name: string;
}

type ProviderConfig =
  | { type: "Folder"; sync_dir: string }
  | { type: "Hosted"; api_url: string; api_key: string };

const DEFAULT_CONFIG: AppConfig = {
  steelseries_db_path: "",
  backup_dir: "",
  max_backups: 20,
  debounce_secs: 3,
  provider: { type: "Folder", sync_dir: "" },
  device_name: "",
};

export default function Settings() {
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);
  const [providerType, setProviderType] = useState<"Folder" | "Hosted">("Folder");
  const [syncDir, setSyncDir] = useState("");
  const [apiUrl, setApiUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null);

  useEffect(() => {
    loadConfig();
  }, []);

  async function loadConfig() {
    try {
      const json = await invoke<string>("get_config");
      const loaded: AppConfig = JSON.parse(json);
      setConfig(loaded);
      if (loaded.provider.type === "Folder") {
        setProviderType("Folder");
        setSyncDir(loaded.provider.sync_dir);
      } else {
        setProviderType("Hosted");
        setApiUrl(loaded.provider.api_url);
        setApiKey(loaded.provider.api_key);
      }
    } catch {
      // Config not available yet (backend not ready), use defaults
    }
  }

  async function saveConfig() {
    setSaving(true);
    setMessage(null);

    const provider: ProviderConfig =
      providerType === "Folder"
        ? { type: "Folder", sync_dir: syncDir }
        : { type: "Hosted", api_url: apiUrl, api_key: apiKey };

    const updated: AppConfig = {
      ...config,
      provider,
    };

    try {
      await invoke("save_config", { config: updated });
      setMessage({ text: "Settings saved.", error: false });
    } catch (err) {
      setMessage({ text: String(err), error: true });
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="page">
      <h2>Settings</h2>

      <div className="form-group">
        <label htmlFor="ss-path">SteelSeries Config Path</label>
        <input
          id="ss-path"
          type="text"
          value={config.steelseries_db_path}
          onChange={(e) =>
            setConfig({ ...config, steelseries_db_path: e.target.value })
          }
          placeholder="/path/to/SteelSeries Engine 3/db"
        />
      </div>

      <div className="form-group">
        <label htmlFor="provider">Sync Provider</label>
        <select
          id="provider"
          value={providerType}
          onChange={(e) => setProviderType(e.target.value as "Folder" | "Hosted")}
        >
          <option value="Folder">Folder (Dropbox, OneDrive, etc.)</option>
          <option value="Hosted">Hosted API</option>
        </select>
      </div>

      {providerType === "Folder" && (
        <div className="form-group">
          <label htmlFor="sync-dir">Sync Folder Path</label>
          <input
            id="sync-dir"
            type="text"
            value={syncDir}
            onChange={(e) => setSyncDir(e.target.value)}
            placeholder="/path/to/sync/folder"
          />
        </div>
      )}

      {providerType === "Hosted" && (
        <>
          <div className="form-group">
            <label htmlFor="api-url">API URL</label>
            <input
              id="api-url"
              type="text"
              value={apiUrl}
              onChange={(e) => setApiUrl(e.target.value)}
              placeholder="https://sync.example.com"
            />
          </div>
          <div className="form-group">
            <label htmlFor="api-key">API Key</label>
            <input
              id="api-key"
              type="text"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="your-api-key"
            />
          </div>
        </>
      )}

      <div className="form-group">
        <label htmlFor="device-name">Device Name</label>
        <input
          id="device-name"
          type="text"
          value={config.device_name}
          onChange={(e) =>
            setConfig({ ...config, device_name: e.target.value })
          }
          placeholder="my-gaming-pc"
        />
      </div>

      <div className="form-row">
        <div className="form-group">
          <label htmlFor="max-backups">Max Backups</label>
          <input
            id="max-backups"
            type="number"
            min={1}
            max={100}
            value={config.max_backups}
            onChange={(e) =>
              setConfig({ ...config, max_backups: parseInt(e.target.value) || 20 })
            }
          />
        </div>
        <div className="form-group">
          <label htmlFor="debounce">Debounce (seconds)</label>
          <input
            id="debounce"
            type="number"
            min={1}
            max={30}
            value={config.debounce_secs}
            onChange={(e) =>
              setConfig({ ...config, debounce_secs: parseInt(e.target.value) || 3 })
            }
          />
        </div>
      </div>

      <button className="btn btn-primary" onClick={saveConfig} disabled={saving}>
        {saving ? "Saving..." : "Save Settings"}
      </button>

      {message && (
        <div
          className={`message ${message.error ? "message-error" : "message-success"}`}
          style={{ marginTop: 16 }}
        >
          {message.text}
        </div>
      )}
    </div>
  );
}
