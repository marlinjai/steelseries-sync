//! File system watcher with debouncing for SteelSeries config changes.

use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct ConfigWatcher {
    config_dir: PathBuf,
    debounce_duration: Duration,
}

/// Event emitted when a debounced config change is detected.
#[derive(Debug, Clone)]
pub struct ConfigChanged {
    pub config_dir: PathBuf,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ConfigWatcher {
    pub fn new(config_dir: PathBuf, debounce_secs: u64) -> Self {
        Self {
            config_dir,
            debounce_duration: Duration::from_secs(debounce_secs),
        }
    }

    /// Start watching. Calls `on_change` after debounced changes to database.db* files.
    /// This blocks the current thread. Run in a dedicated thread.
    pub fn watch<F>(&self, on_change: F) -> notify::Result<()>
    where
        F: Fn(ConfigChanged) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if is_db_event(&event) {
                    let _ = tx.send(event);
                }
            }
        })?;

        watcher.watch(&self.config_dir, RecursiveMode::NonRecursive)?;

        let debounce = self.debounce_duration;
        let config_dir = self.config_dir.clone();

        // Debounce loop
        let mut last_event: Option<Instant> = None;
        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(_) => {
                    last_event = Some(Instant::now());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(last) = last_event {
                        if last.elapsed() >= debounce {
                            last_event = None;
                            on_change(ConfigChanged {
                                config_dir: config_dir.clone(),
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(())
    }
}

/// Check if a notify event relates to database.db* files.
fn is_db_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_)
    ) && event.paths.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("database.db"))
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_event(path: &str, kind: EventKind) -> Event {
        Event {
            kind,
            paths: vec![PathBuf::from(path)],
            attrs: Default::default(),
        }
    }

    #[test]
    fn test_is_db_event_matches_database_files() {
        let event = mock_event("/some/path/database.db", EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)));
        assert!(is_db_event(&event));

        let event = mock_event("/some/path/database.db-wal", EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)));
        assert!(is_db_event(&event));

        let event = mock_event("/some/path/database.db-shm", EventKind::Create(notify::event::CreateKind::File));
        assert!(is_db_event(&event));
    }

    #[test]
    fn test_is_db_event_ignores_unrelated_files() {
        let event = mock_event("/some/path/config.json", EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)));
        assert!(!is_db_event(&event));
    }

    #[test]
    fn test_is_db_event_ignores_delete_events() {
        let event = mock_event("/some/path/database.db", EventKind::Remove(notify::event::RemoveKind::File));
        assert!(!is_db_event(&event));
    }
}
