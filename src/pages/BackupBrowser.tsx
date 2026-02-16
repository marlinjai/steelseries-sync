import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function BackupBrowser() {
  const [backups, setBackups] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [restoring, setRestoring] = useState<string | null>(null);
  const [message, setMessage] = useState<{ text: string; error: boolean } | null>(null);

  useEffect(() => {
    fetchBackups();
  }, []);

  async function fetchBackups() {
    setLoading(true);
    try {
      const list = await invoke<string[]>("list_backups");
      setBackups(list);
    } catch {
      // Backend not ready yet, show empty state
      setBackups([]);
    } finally {
      setLoading(false);
    }
  }

  async function restoreBackup(name: string) {
    const confirmed = window.confirm(
      `Restore backup "${name}"?\n\nThis will overwrite your current SteelSeries config with this backup. A new backup of the current config will be created first.`
    );
    if (!confirmed) return;

    setRestoring(name);
    setMessage(null);
    try {
      await invoke("restore_backup", { backupName: name });
      setMessage({ text: `Restored backup "${name}" successfully.`, error: false });
      await fetchBackups();
    } catch (err) {
      setMessage({ text: String(err), error: true });
    } finally {
      setRestoring(null);
    }
  }

  function parseBackupDate(name: string): string {
    // Backup names are like "pre-pull-2026-02-16T14-30-00" or "sync-2026-02-16T14-30-00"
    const match = name.match(/(\d{4}-\d{2}-\d{2})T(\d{2})-(\d{2})-(\d{2})$/);
    if (match) {
      const [, date, h, m, s] = match;
      return `${date} ${h}:${m}:${s}`;
    }
    return "";
  }

  if (loading) {
    return (
      <div className="page">
        <h2>Backups</h2>
        <div className="empty-state">Loading backups...</div>
      </div>
    );
  }

  return (
    <div className="page">
      <h2>Backups</h2>

      <div style={{ marginBottom: 16 }}>
        <button className="btn btn-secondary btn-sm" onClick={fetchBackups}>
          Refresh
        </button>
      </div>

      {message && (
        <div
          className={`message ${message.error ? "message-error" : "message-success"}`}
          style={{ marginBottom: 16 }}
        >
          {message.text}
        </div>
      )}

      {backups.length === 0 ? (
        <div className="empty-state">
          No backups yet. Backups are created automatically before sync operations.
        </div>
      ) : (
        <div className="backup-list">
          {backups.map((name) => (
            <div key={name} className="backup-item">
              <div className="backup-info">
                <span className="backup-name">{name}</span>
                <span className="backup-date">{parseBackupDate(name)}</span>
              </div>
              <button
                className="btn btn-danger btn-sm"
                onClick={() => restoreBackup(name)}
                disabled={restoring !== null}
              >
                {restoring === name ? "Restoring..." : "Restore"}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
