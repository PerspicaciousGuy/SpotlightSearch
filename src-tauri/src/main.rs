#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use gleam_search::{icons, Index, IndexStatus, SearchResult, APPS_FOLDER};
use std::sync::Arc;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    window::{Color, Effect, EffectsBuilder},
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
// The blur setting is a marker file so it survives restarts without a settings system.
// Blur is on unless the file exists.
fn blur_marker(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join("blur-off"))
}
fn blur_enabled_now(app: &tauri::AppHandle) -> bool {
    !blur_marker(app).is_some_and(|marker| marker.exists())
}
// Acrylic fills the whole native window, so the page drops its transparent margin and
// relies on the system shadow and rounded corners while blur is on. See App.tsx.
fn apply_blur(app: &tauri::AppHandle, on: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let effects = on.then(|| EffectsBuilder::new().effect(Effect::Acrylic).color(Color(244, 244, 250, 110)).build());
        let _ = window.set_effects(effects);
        let _ = window.set_shadow(on);
        let _ = window.emit("blur-changed", on);
    }
}
#[tauri::command]
fn blur_enabled(app: tauri::AppHandle) -> bool {
    blur_enabled_now(&app)
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
// Icons are requested at twice the displayed size so they stay sharp on high-DPI screens.
const ICON_SIZE: i32 = 64;
#[tauri::command]
async fn app_icon(
    app: tauri::AppHandle,
    index: State<'_, Arc<Index>>,
    id: i64,
) -> Result<Option<String>, String> {
    let index = index.inner().clone();
    let cache = app.path().app_data_dir().ok().map(|dir| dir.join("icons"));
    tauri::async_runtime::spawn_blocking(move || {
        let path = index.path_for_id(id)?;
        Ok(icons::cached_icon(&path, cache.as_deref(), ICON_SIZE, icons::shell_icon_png))
    })
    .await
    .map_err(|e| e.to_string())?
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
        // Store/MSIX apps have no file path; the shell launches them from the apps folder.
        if path.to_str().is_some_and(|p| p.starts_with(APPS_FOLDER)) {
            return std::process::Command::new("explorer.exe")
                .arg(&path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
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
        .invoke_handler(tauri::generate_handler![search, index_status, rebuild_index, open_result, blur_enabled, app_icon])
        .setup(|app| {
            let index = Index::new(app.path().app_data_dir()?.join("index.sqlite3"))?;
            app.manage(index.clone());
            index.rebuild_system();
            // A shortcut conflict is surfaced to the user; the tray can still open the app.
            if let Err(error) = app.global_shortcut().register("Alt+Space") {
                let message = format!("Alt + Space could not be registered: {error}. Open Gleam from the system tray.");
                app.manage(ShortcutError(message));
            }
            apply_blur(app.handle(), blur_enabled_now(app.handle()));
            let blur = CheckMenuItem::with_id(app, "blur", "Blur background", true, blur_enabled_now(app.handle()), None::<&str>)?;
            let open = MenuItem::with_id(app, "open", "Open Gleam", true, None::<&str>)?;
            let refresh = MenuItem::with_id(app, "refresh", "Refresh index", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Gleam", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &refresh, &blur, &quit])?;
            let icon = tauri::image::Image::new_owned(tray_pixels(), 32, 32);
            TrayIconBuilder::new().icon(icon).tooltip("Gleam · Alt + Space").menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open" => show(app),
                    "refresh" => app.state::<Arc<Index>>().inner().rebuild_system(),
                    "blur" => {
                        let on = !blur_enabled_now(app);
                        if let Some(marker) = blur_marker(app) {
                            if on { let _ = std::fs::remove_file(marker); } else { let _ = std::fs::write(marker, ""); }
                        }
                        let _ = blur.set_checked(on);
                        apply_blur(app, on);
                    }
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
        .run(tauri::generate_context!()).expect("Unable to start Gleam");
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
