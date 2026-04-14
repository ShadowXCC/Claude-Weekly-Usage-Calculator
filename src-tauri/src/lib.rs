mod icon;
mod usage;

use std::sync::Mutex;
use std::time::Duration;

use chrono::Local;
use serde::Serialize;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use tauri_plugin_store::StoreExt;

use usage::ResetConfig;

// ── State ─────────────────────────────────────────────────────────────────────

pub struct AppState {
    pub config: ResetConfig,
}

impl Default for AppState {
    fn default() -> Self {
        Self { config: ResetConfig::default() }
    }
}


// ── Tauri commands ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct UsageInfo {
    pub suggested_pct: f64,
    pub time_until_reset: String,
    pub reset_weekday: u32,
    pub reset_hour: u32,
    pub reset_minute: u32,
    pub last_refreshed: String,
}

#[tauri::command]
fn get_usage_info(state: tauri::State<'_, Mutex<AppState>>) -> UsageInfo {
    let state = state.lock().unwrap();
    let now = Local::now();
    let (pct, _, _) = usage::suggested_usage(now, &state.config);
    let time_str = usage::time_until_next_reset(now, &state.config);
    let refreshed = now.format("%-I:%M %p").to_string();
    UsageInfo {
        suggested_pct: pct,
        time_until_reset: time_str,
        reset_weekday: state.config.weekday,
        reset_hour: state.config.hour,
        reset_minute: state.config.minute,
        last_refreshed: refreshed,
    }
}


#[tauri::command]
fn save_config(
    weekday: u32,
    hour: u32,
    minute: u32,
    run_on_startup: bool,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    if weekday > 6 { return Err("Invalid weekday".into()); }
    if hour > 23   { return Err("Invalid hour".into()); }
    if minute > 59 { return Err("Invalid minute".into()); }

    let cfg = ResetConfig { weekday, hour, minute };

    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("weekday", serde_json::json!(weekday));
    store.set("hour", serde_json::json!(hour));
    store.set("minute", serde_json::json!(minute));
    store.set("run_on_startup", serde_json::json!(run_on_startup));
    store.save().map_err(|e| e.to_string())?;

    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autostart = app.autolaunch();
        if run_on_startup { let _ = autostart.enable(); }
        else              { let _ = autostart.disable(); }
    }

    { let mut s = state.lock().unwrap(); s.config = cfg; }

    update_tray_icon(&app);
    Ok(())
}

#[tauri::command]
fn get_startup_enabled(app: AppHandle) -> bool {
    #[cfg(desktop)]
    { use tauri_plugin_autostart::ManagerExt; app.autolaunch().is_enabled().unwrap_or(false) }
    #[cfg(not(desktop))]
    false
}

// ── Tray icon update ──────────────────────────────────────────────────────────

pub fn update_tray_icon(app: &AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    let cfg = { let s = state.lock().unwrap(); s.config.clone() };
    let now = Local::now();
    let (pct, _, _) = usage::suggested_usage(now, &cfg);
    let fill = (pct / 100.0).clamp(0.0, 1.0);

    let png_bytes = icon::render_tray_icon(fill, 64);
    let img = tauri::image::Image::from_bytes(&png_bytes).expect("Failed to create tray image");

    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = format!("Claude Usage: {:.1}% suggested", pct);
        let _ = tray.set_icon(Some(img));
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

// ── App entry point ───────────────────────────────────────────────────────────

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let store = app.store("settings.json")?;
            let cfg = ResetConfig {
                weekday: store.get("weekday").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(6),
                hour:    store.get("hour").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(0),
                minute:  store.get("minute").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(0),
            };

            let has_startup_pref = store.get("run_on_startup").is_some();
            #[cfg(desktop)]
            if !has_startup_pref {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
                store.set("run_on_startup", serde_json::json!(true));
                let _ = store.save();
            }

            app.manage(Mutex::new(AppState { config: cfg, ..Default::default() }));

            let open_item     = MenuItemBuilder::with_id("open_app",      "Open Claude Usage Tracker").build(app)?;
            let usage_item    = MenuItemBuilder::with_id("open_usage",    "Claude Usage Page").build(app)?;
            let settings_item = MenuItemBuilder::with_id("open_settings", "Settings").build(app)?;
            let quit_item     = MenuItemBuilder::with_id("quit",          "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&open_item, &usage_item, &settings_item, &quit_item])
                .build()?;

            let initial_img = tauri::image::Image::from_bytes(&icon::render_tray_icon(0.0, 64))?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(initial_img)
                .tooltip("Claude Usage Tracker")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_app"      => show_main_window(app),
                    "open_usage"    => { let _ = tauri_plugin_opener::open_url("https://claude.ai/settings/usage", None::<&str>); }
                    "open_settings" => show_main_window(app),
                    "quit"          => app.exit(0),
                    _               => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(60));
                update_tray_icon(&app_handle);
            });

            update_tray_icon(app.handle());

            if let Some(win) = app.get_webview_window("main") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
                let _ = win.hide();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_usage_info,
            save_config,
            get_startup_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}
