mod icon;
pub mod usage;

use std::sync::Mutex;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use chrono_tz::TZ_VARIANTS;
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_store::StoreExt;

use icon::TrayMode;
use usage::ResetConfig;

// ── State ─────────────────────────────────────────────────────────────────────

pub struct AppState {
    pub config: ResetConfig,
    pub display: DisplayPrefs,
    pub threshold: ThresholdTracker,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: ResetConfig::default(),
            display: DisplayPrefs::default(),
            threshold: ThresholdTracker::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayPrefs {
    pub display_inverse: bool,
    pub tray_mode: String,             // "ring" | "number" | "emoji"
    pub accent_hex: String,            // "#RRGGBB"
    pub a11y_announce_thresholds: bool,
}

impl Default for DisplayPrefs {
    fn default() -> Self {
        Self {
            display_inverse: false,
            tray_mode: "ring".to_string(),
            accent_hex: "#DA7756".to_string(),
            a11y_announce_thresholds: true,
        }
    }
}

/// Bitmask tracking which thresholds (50/75/90/100) have been announced in the
/// current weekly cycle. Resets when we detect pct has dropped (i.e. the week
/// rolled over since the last tick).
#[derive(Debug, Default)]
pub struct ThresholdTracker {
    pub last_pct: f64,
    pub announced_mask: u32,
}

const THRESHOLDS: &[(u32, f64)] = &[(1, 50.0), (2, 75.0), (4, 90.0), (8, 100.0)];

impl ThresholdTracker {
    /// Returns the highest newly-crossed threshold (or None). Updates state.
    pub fn step(&mut self, new_pct: f64) -> Option<u32> {
        // Reset detection: pct fell significantly → a new week started.
        if new_pct + 5.0 < self.last_pct {
            self.announced_mask = 0;
        }
        self.last_pct = new_pct;

        let mut highest_new: Option<u32> = None;
        for &(bit, threshold) in THRESHOLDS {
            if new_pct >= threshold && (self.announced_mask & bit) == 0 {
                self.announced_mask |= bit;
                highest_new = Some(threshold as u32);
            }
        }
        highest_new
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct UsageInfo {
    pub suggested_pct: f64,
    pub remaining_pct: f64,
    pub time_until_reset: String,
    pub reset_weekday: u32,
    pub reset_hour: u32,
    pub reset_minute: u32,
    pub last_refreshed: String,
    pub sleep_start_hour: u32,
    pub sleep_start_minute: u32,
    pub sleep_end_hour: u32,
    pub sleep_end_minute: u32,
    pub sleep_adjustment_enabled: bool,
    pub tz: String,          // "auto" or IANA zone name
    pub tz_display: String,  // resolved: "Local" or IANA zone name
    pub display_inverse: bool,
    pub tray_mode: String,
    pub accent_hex: String,
    pub a11y_announce_thresholds: bool,
    pub now_unix: i64,
    pub next_reset_unix: i64,
}

fn build_usage_info(cfg: &ResetConfig, display: &DisplayPrefs) -> UsageInfo {
    let (pct, _, _) = usage::current_usage(cfg);
    let time_str = usage::current_time_until_reset(cfg);
    let now_unix = Utc::now().timestamp();
    let next_reset = usage::next_reset_unix(cfg);
    // Use the human wall-clock label in whichever zone the user picked.
    let refreshed = human_clock_in_tz(cfg);

    UsageInfo {
        suggested_pct: pct,
        remaining_pct: (100.0 - pct).max(0.0),
        time_until_reset: time_str,
        reset_weekday: cfg.weekday,
        reset_hour: cfg.hour,
        reset_minute: cfg.minute,
        last_refreshed: refreshed,
        sleep_start_hour: cfg.sleep_start_hour,
        sleep_start_minute: cfg.sleep_start_minute,
        sleep_end_hour: cfg.sleep_end_hour,
        sleep_end_minute: cfg.sleep_end_minute,
        sleep_adjustment_enabled: cfg.sleep_adjustment_enabled,
        tz: cfg.tz.clone().unwrap_or_else(|| "auto".to_string()),
        tz_display: cfg.tz_display_name(),
        display_inverse: display.display_inverse,
        tray_mode: display.tray_mode.clone(),
        accent_hex: display.accent_hex.clone(),
        a11y_announce_thresholds: display.a11y_announce_thresholds,
        now_unix,
        next_reset_unix: next_reset,
    }
}

fn human_clock_in_tz(cfg: &ResetConfig) -> String {
    use usage::ConfiguredTz;
    match cfg.resolve_tz() {
        ConfiguredTz::Local => chrono::Local::now().format("%-I:%M %p").to_string(),
        ConfiguredTz::Fixed(tz) => Utc::now().with_timezone(&tz).format("%-I:%M %p").to_string(),
    }
}

#[tauri::command]
fn get_usage_info(state: tauri::State<'_, Mutex<AppState>>) -> UsageInfo {
    let s = state.lock().unwrap();
    build_usage_info(&s.config, &s.display)
}

#[derive(Serialize)]
pub struct SimulationResult {
    pub suggested_pct: f64,
    pub remaining_pct: f64,
    pub target_label: String,
}

#[tauri::command]
fn simulate_at(unix_seconds: i64, state: tauri::State<'_, Mutex<AppState>>) -> SimulationResult {
    let cfg = { state.lock().unwrap().config.clone() };
    let (pct, _, _) = usage::simulate_at_unix(unix_seconds, &cfg);
    let target_label = {
        use usage::ConfiguredTz;
        let utc = chrono::Utc
            .timestamp_opt(unix_seconds, 0)
            .single()
            .unwrap_or_else(chrono::Utc::now);
        match cfg.resolve_tz() {
            ConfiguredTz::Local => utc
                .with_timezone(&chrono::Local)
                .format("%a %-I:%M %p")
                .to_string(),
            ConfiguredTz::Fixed(tz) => utc
                .with_timezone(&tz)
                .format("%a %-I:%M %p")
                .to_string(),
        }
    };
    SimulationResult {
        suggested_pct: pct,
        remaining_pct: (100.0 - pct).max(0.0),
        target_label,
    }
}

#[tauri::command]
fn get_timezones() -> Vec<String> {
    TZ_VARIANTS.iter().map(|tz| tz.name().to_string()).collect()
}

#[tauri::command]
fn save_config(
    weekday: u32,
    hour: u32,
    minute: u32,
    run_on_startup: bool,
    sleep_start_hour: u32,
    sleep_start_minute: u32,
    sleep_end_hour: u32,
    sleep_end_minute: u32,
    sleep_adjustment_enabled: bool,
    timezone: Option<String>,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    if weekday > 6 { return Err("Invalid weekday".into()); }
    if hour > 23   { return Err("Invalid hour".into()); }
    if minute > 59 { return Err("Invalid minute".into()); }
    if sleep_start_hour > 23 || sleep_end_hour > 23 {
        return Err("Invalid sleep hour".into());
    }
    if sleep_start_minute > 59 || sleep_end_minute > 59 {
        return Err("Invalid sleep minute".into());
    }

    let start = chrono::NaiveTime::from_hms_opt(sleep_start_hour, sleep_start_minute, 0)
        .ok_or("Invalid sleep start time")?;
    let end = chrono::NaiveTime::from_hms_opt(sleep_end_hour, sleep_end_minute, 0)
        .ok_or("Invalid sleep end time")?;
    let daily_sleep_hours = usage::daily_sleep_duration_secs(start, end) / 3600.0;
    if daily_sleep_hours > 16.0 {
        return Err("Sleep window cannot exceed 16 hours".into());
    }

    // Validate timezone string: must be empty/auto or a known IANA zone.
    let tz_sanitized = match timezone.as_deref() {
        None | Some("") | Some("auto") => None,
        Some(name) => {
            if name.parse::<chrono_tz::Tz>().is_err() {
                return Err(format!("Unknown timezone: {}", name));
            }
            Some(name.to_string())
        }
    };

    let cfg = ResetConfig {
        weekday,
        hour,
        minute,
        sleep_start_hour,
        sleep_start_minute,
        sleep_end_hour,
        sleep_end_minute,
        sleep_adjustment_enabled,
        tz: tz_sanitized.clone(),
    };

    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("weekday", serde_json::json!(weekday));
    store.set("hour", serde_json::json!(hour));
    store.set("minute", serde_json::json!(minute));
    store.set("run_on_startup", serde_json::json!(run_on_startup));
    store.set("sleep_start_hour", serde_json::json!(sleep_start_hour));
    store.set("sleep_start_minute", serde_json::json!(sleep_start_minute));
    store.set("sleep_end_hour", serde_json::json!(sleep_end_hour));
    store.set("sleep_end_minute", serde_json::json!(sleep_end_minute));
    store.set("sleep_adjustment_enabled", serde_json::json!(sleep_adjustment_enabled));
    store.set("timezone", serde_json::json!(tz_sanitized.clone().unwrap_or_else(|| "auto".to_string())));
    store.save().map_err(|e| e.to_string())?;

    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autostart = app.autolaunch();
        if run_on_startup { let _ = autostart.enable(); }
        else              { let _ = autostart.disable(); }
    }

    {
        let mut s = state.lock().unwrap();
        s.config = cfg;
        // Changing the reset schedule means the old threshold state is meaningless.
        s.threshold = ThresholdTracker::default();
    }

    update_tray_icon(&app);
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct DisplayPrefsInput {
    pub display_inverse: Option<bool>,
    pub tray_mode: Option<String>,
    pub accent_hex: Option<String>,
    pub a11y_announce_thresholds: Option<bool>,
}

#[tauri::command]
fn update_display_prefs(
    prefs: DisplayPrefsInput,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    // Merge into existing prefs.
    let new_display = {
        let s = state.lock().unwrap();
        let mut d = s.display.clone();
        if let Some(v) = prefs.display_inverse { d.display_inverse = v; }
        if let Some(v) = prefs.tray_mode {
            if !matches!(v.as_str(), "ring" | "number" | "emoji") {
                return Err(format!("Unknown tray_mode: {}", v));
            }
            d.tray_mode = v;
        }
        if let Some(v) = prefs.accent_hex {
            if !is_valid_hex(&v) {
                return Err(format!("Invalid accent hex: {}", v));
            }
            d.accent_hex = v;
        }
        if let Some(v) = prefs.a11y_announce_thresholds { d.a11y_announce_thresholds = v; }
        d
    };

    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("display_inverse", serde_json::json!(new_display.display_inverse));
    store.set("tray_mode", serde_json::json!(new_display.tray_mode));
    store.set("accent_hex", serde_json::json!(new_display.accent_hex));
    store.set("a11y_announce_thresholds", serde_json::json!(new_display.a11y_announce_thresholds));
    store.save().map_err(|e| e.to_string())?;

    {
        let mut s = state.lock().unwrap();
        s.display = new_display;
    }

    update_tray_icon(&app);
    Ok(())
}

fn is_valid_hex(s: &str) -> bool {
    let s = s.trim_start_matches('#');
    s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit())
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
    let (cfg, display) = {
        let state = app.state::<Mutex<AppState>>();
        let s = state.lock().unwrap();
        (s.config.clone(), s.display.clone())
    };

    let (pct, _, _) = usage::current_usage(&cfg);
    let remaining = (100.0 - pct).max(0.0);
    let shown = if display.display_inverse { remaining } else { pct };
    let fill = (pct / 100.0).clamp(0.0, 1.0);
    let accent = icon::parse_hex_rgba(&display.accent_hex);
    let mode = TrayMode::from_str(&display.tray_mode);

    let png_bytes = icon::render_tray_icon(
        fill,
        64,
        mode,
        accent,
        shown.round() as u32,
        display.display_inverse,
    );
    let img = tauri::image::Image::from_bytes(&png_bytes).expect("Failed to create tray image");

    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = format!(
            "Claude Usage: {:.1}% used · {:.1}% remaining",
            pct, remaining
        );
        let _ = tray.set_icon(Some(img));
        let _ = tray.set_tooltip(Some(&tooltip));

        // Emoji mode uses the platform's tray text label in addition to the icon.
        // On platforms where set_title is unsupported it's effectively a no-op.
        let title = if mode == TrayMode::Emoji {
            Some(format_emoji_label(pct, shown))
        } else {
            None
        };
        let _ = tray.set_title(title.as_deref());
    }
}

fn format_emoji_label(pct_used: f64, shown: f64) -> String {
    // Color indicates budget health (based on used %, regardless of inverse display).
    let dot = if pct_used < 50.0 {
        "🟢"
    } else if pct_used < 80.0 {
        "🟡"
    } else {
        "🔴"
    };
    format!("{}{}%", dot, shown.round() as u32)
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
                sleep_start_hour:   store.get("sleep_start_hour").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(23),
                sleep_start_minute: store.get("sleep_start_minute").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(0),
                sleep_end_hour:     store.get("sleep_end_hour").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(7),
                sleep_end_minute:   store.get("sleep_end_minute").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(0),
                sleep_adjustment_enabled: store.get("sleep_adjustment_enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                tz: store.get("timezone").and_then(|v| v.as_str().map(String::from))
                    .filter(|s| !s.is_empty() && s != "auto"),
            };

            let display = DisplayPrefs {
                display_inverse: store.get("display_inverse").and_then(|v| v.as_bool()).unwrap_or(false),
                tray_mode: store.get("tray_mode").and_then(|v| v.as_str().map(String::from))
                    .filter(|s| matches!(s.as_str(), "ring" | "number" | "emoji"))
                    .unwrap_or_else(|| "ring".to_string()),
                accent_hex: store.get("accent_hex").and_then(|v| v.as_str().map(String::from))
                    .filter(|s| is_valid_hex(s))
                    .unwrap_or_else(|| "#DA7756".to_string()),
                a11y_announce_thresholds: store.get("a11y_announce_thresholds").and_then(|v| v.as_bool()).unwrap_or(true),
            };

            let has_startup_pref = store.get("run_on_startup").is_some();
            #[cfg(desktop)]
            if !has_startup_pref {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
                store.set("run_on_startup", serde_json::json!(true));
                let _ = store.save();
            }

            app.manage(Mutex::new(AppState { config: cfg, display, threshold: ThresholdTracker::default() }));

            let open_item     = MenuItemBuilder::with_id("open_app",      "Open Claude Weekly Usage Calculator").build(app)?;
            let usage_item    = MenuItemBuilder::with_id("open_usage",    "Claude Usage Page").build(app)?;
            let settings_item = MenuItemBuilder::with_id("open_settings", "Settings").build(app)?;
            let quit_item     = MenuItemBuilder::with_id("quit",          "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&open_item, &usage_item, &settings_item, &quit_item])
                .build()?;

            let initial_img = tauri::image::Image::from_bytes(&icon::render_tray_icon(
                0.0, 64, TrayMode::Ring, icon::parse_hex_rgba("#DA7756"), 0, false,
            ))?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(initial_img)
                .tooltip("Claude Weekly Usage Calculator")
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
                emit_threshold_if_crossed(&app_handle);
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
            get_timezones,
            simulate_at,
            update_display_prefs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn emit_threshold_if_crossed(app: &AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    let (crossed, announce) = {
        let mut s = state.lock().unwrap();
        let (pct, _, _) = usage::current_usage(&s.config);
        let announce = s.display.a11y_announce_thresholds;
        let crossed = s.threshold.step(pct);
        (crossed, announce)
    };
    if announce {
        if let Some(threshold) = crossed {
            let _ = app.emit("threshold-crossed", threshold);
        }
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}
