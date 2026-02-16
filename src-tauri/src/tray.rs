//! System tray setup with menu items for quick actions.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

/// Set up the system tray with menu items.
/// Call this from the Tauri setup hook in lib.rs.
pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let sync_now = MenuItem::with_id(app, "sync_now", "Sync Now", true, None::<&str>)?;
    let open_window = MenuItem::with_id(app, "open_window", "Open Window", true, None::<&str>)?;
    let view_backups =
        MenuItem::with_id(app, "view_backups", "View Backups", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&sync_now, &open_window, &view_backups, &separator, &quit],
    )?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .tooltip("SteelSeries Sync")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "sync_now" => {
                // Emit an event that the frontend can listen for
                let _ = app.emit("tray-sync-now", ());
            }
            "open_window" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "view_backups" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = app.emit("tray-view-backups", ());
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
