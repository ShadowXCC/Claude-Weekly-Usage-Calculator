//! `claude-usage` — CLI companion that prints the current suggested weekly
//! usage percent to stdout, sharing configuration with the GUI app.
//!
//! Reads `settings.json` from the same location tauri-plugin-store writes it
//! (`$CONFIG_DIR/dev.claude.usage-tracker/settings.json`), so "install the app,
//! run `claude-usage`" works immediately.

use clap::Parser;
use claude_weekly_usage_calculator_lib::usage::{self, ResetConfig};
use serde_json::Value;
use std::path::PathBuf;
use std::process::ExitCode;

const APP_IDENTIFIER: &str = "dev.claude.usage-tracker";

#[derive(Parser, Debug)]
#[command(
    name = "claude-usage",
    about = "Print current Claude weekly suggested-usage percent",
    version
)]
struct Args {
    /// Force 'used' percent (overrides the GUI's inverse setting).
    #[arg(long, conflicts_with = "remaining")]
    used: bool,

    /// Force 'remaining' percent (overrides the GUI's inverse setting).
    #[arg(long, conflicts_with = "used")]
    remaining: bool,

    /// Emit JSON with all fields.
    #[arg(long, conflicts_with_all = ["format"])]
    json: bool,

    /// Template string. Available placeholders: {pct} {used} {remaining}
    /// {reset_in} {tz}. Example: --format '{pct}% used, {reset_in} left'
    #[arg(long)]
    format: Option<String>,

    /// Number of fractional digits in default output (default: 1).
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(0..=4))]
    precision: u8,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let cfg = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("claude-usage: {}", e);
            return ExitCode::from(1);
        }
    };

    let (used, _, _) = usage::current_usage(&cfg);
    let remaining = (100.0 - used).max(0.0);
    let reset_in = usage::current_time_until_reset(&cfg);
    let tz_name = cfg.tz_display_name();

    // Read the GUI's inverse setting if neither --used nor --remaining was passed.
    let inverse_default = load_display_inverse().unwrap_or(false);
    let show_remaining = if args.remaining {
        true
    } else if args.used {
        false
    } else {
        inverse_default
    };
    let pct = if show_remaining { remaining } else { used };

    if args.json {
        let obj = serde_json::json!({
            "used": round(used, args.precision),
            "remaining": round(remaining, args.precision),
            "pct": round(pct, args.precision),
            "inverse": show_remaining,
            "reset_in": reset_in,
            "tz": tz_name,
        });
        println!("{}", obj);
        return ExitCode::SUCCESS;
    }

    if let Some(tmpl) = args.format {
        let out = tmpl
            .replace("{pct}", &format_float(pct, args.precision))
            .replace("{used}", &format_float(used, args.precision))
            .replace("{remaining}", &format_float(remaining, args.precision))
            .replace("{reset_in}", &reset_in)
            .replace("{tz}", &tz_name);
        println!("{}", out);
        return ExitCode::SUCCESS;
    }

    println!("{}", format_float(pct, args.precision));
    ExitCode::SUCCESS
}

fn round(v: f64, precision: u8) -> f64 {
    let p = 10_f64.powi(precision as i32);
    (v * p).round() / p
}

fn format_float(v: f64, precision: u8) -> String {
    format!("{:.*}", precision as usize, v)
}

/// Resolves the config dir the same way tauri-plugin-store does internally:
/// `dirs::config_dir() / APP_IDENTIFIER / settings.json`.
///
/// - Linux:   ~/.config/dev.claude.usage-tracker/settings.json
/// - macOS:   ~/Library/Application Support/dev.claude.usage-tracker/settings.json
/// - Windows: %APPDATA%/dev.claude.usage-tracker/settings.json
fn settings_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(APP_IDENTIFIER).join("settings.json"))
}

fn load_config() -> Result<ResetConfig, String> {
    let path = settings_path().ok_or_else(|| {
        "could not determine config directory. Set XDG_CONFIG_HOME or HOME.".to_string()
    })?;
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "could not read {}: {}. Open the GUI once to create it.",
            path.display(),
            e
        )
    })?;
    let v: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("settings.json is not valid JSON: {}", e))?;

    Ok(ResetConfig {
        weekday: v.get("weekday").and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(6),
        hour: v.get("hour").and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(0),
        minute: v.get("minute").and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(0),
        sleep_start_hour: v.get("sleep_start_hour").and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(23),
        sleep_start_minute: v.get("sleep_start_minute").and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(0),
        sleep_end_hour: v.get("sleep_end_hour").and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(7),
        sleep_end_minute: v.get("sleep_end_minute").and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(0),
        sleep_adjustment_enabled: v.get("sleep_adjustment_enabled").and_then(|x| x.as_bool()).unwrap_or(false),
        tz: v.get("timezone").and_then(|x| x.as_str()).map(String::from)
            .filter(|s| !s.is_empty() && s != "auto"),
    })
}

fn load_display_inverse() -> Option<bool> {
    let path = settings_path()?;
    let raw = std::fs::read_to_string(&path).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    v.get("display_inverse").and_then(|x| x.as_bool())
}
