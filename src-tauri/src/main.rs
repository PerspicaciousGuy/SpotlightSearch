#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use lumen_search::{Index, IndexStatus, SearchResult};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, State,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.emit("launcher-open", ());
    }
}
#[tauri::command]
async fn search(
    index: State<'_, Arc<Index>>,
    query: String,
    kind: String,
) -> Result<Vec<SearchResult>, String> {
    let index = index.inner().clone();
    tauri::async_runtime::spawn_blocking(move || index.search(&query, &kind))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn index_status(index: State<'_, Arc<Index>>) -> IndexStatus {
    index.status()
}
#[tauri::command]
fn rebuild_index(index: State<'_, Arc<Index>>) {
    index.inner().rebuild_system();
}
#[tauri::command]
async fn open_result(
    app: tauri::AppHandle,
    index: State<'_, Arc<Index>>,
    id: i64,
) -> Result<(), String> {
    let index = index.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = index.path_for_id(id)?;
        if !path.exists() {
            return Err(
                "This item has moved or was deleted. Refresh the index and try again.".into(),
            );
        }
        open::that_detached(path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    Ok(())
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, _, event| {
            if event.state() == ShortcutState::Pressed {
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) { let _ = window.hide(); }
                    else { show(app); }
                }
            }
        }).build())
        .invoke_handler(tauri::generate_handler![search, index_status, rebuild_index, open_result])
        .setup(|app| {
            let index = Index::new(app.path().app_data_dir()?.join("index.sqlite3"))?;
            app.manage(index.clone());
            index.rebuild_system();
            // A shortcut conflict is surfaced to the user; the tray can still open the app.
            if let Err(error) = app.global_shortcut().register("Alt+Space") {
                let message = format!("Alt + Space could not be registered: {error}. Open Lumen from the system tray.");
                app.manage(ShortcutError(message));
            }
            let open = MenuItem::with_id(app, "open", "Open Lumen", true, None::<&str>)?;
            let refresh = MenuItem::with_id(app, "refresh", "Refresh index", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Lumen", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &refresh, &quit])?;
            let icon = tauri::image::Image::new_owned(tray_pixels(), 32, 32);
            TrayIconBuilder::new().icon(icon).tooltip("Lumen · Alt + Space").menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show(app),
                    "refresh" => app.state::<Arc<Index>>().inner().rebuild_system(),
                    "quit" => app.exit(0),
                    _ => {}
                }).build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); }
            // A launcher should dismiss itself when the user clicks or switches elsewhere.
            if let tauri::WindowEvent::Focused(false) = event { let _ = window.hide(); }
            if let tauri::WindowEvent::Focused(true) = event {
                if let Some(error) = window.app_handle().try_state::<ShortcutError>() { let _ = window.emit("shortcut-error", &error.0); }
            }
        })
        .run(tauri::generate_context!()).expect("Unable to start Lumen");
}
struct ShortcutError(String);
fn tray_pixels() -> Vec<u8> {
    let mut data = vec![0; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let ring = ((x as i32 - 13).pow(2) + (y as i32 - 13).pow(2) - 64).abs() < 24;
            let handle =
                (20..29).contains(&x) && (20..29).contains(&y) && (x as i32 - y as i32).abs() < 3;
            if ring || handle {
                let i = (y * 32 + x) * 4;
                data[i..i + 4].copy_from_slice(&[125, 211, 191, 255]);
            }
        }
    }
    data
}
