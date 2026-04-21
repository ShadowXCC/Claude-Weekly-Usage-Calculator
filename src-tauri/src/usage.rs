/// Core usage math.
///
/// The Claude usage week is defined as a 7-day rolling window that resets at a
/// user-specified day-of-week and time in a user-chosen timezone (or the OS
/// local zone). The "suggested usage %" at any moment is simply:
///
///   elapsed_seconds / (7 * 24 * 3600) * 100.0
///
/// where elapsed_seconds is the number of seconds since the most recent reset.
/// The math is generic over any `chrono::TimeZone`, so callers can plug in
/// `chrono::Local` (OS local) or a `chrono_tz::Tz` (explicit IANA zone).
/// Sub-minute precision is maintained throughout.

use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz as ChronoTz;
use serde::{Deserialize, Serialize};

/// Day of week (0 = Monday … 6 = Sunday), matching chrono's num_days_from_monday.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResetConfig {
    /// 0 = Monday, 1 = Tuesday, … 6 = Sunday
    pub weekday: u32,
    pub hour: u32,
    pub minute: u32,
    /// Daily sleep window. When `sleep_adjustment_enabled`, the gauge ticks only
    /// during waking hours: the window [start, end) is treated as non-active time.
    /// The window may wrap past midnight (start > end).
    pub sleep_start_hour: u32,
    pub sleep_start_minute: u32,
    pub sleep_end_hour: u32,
    pub sleep_end_minute: u32,
    pub sleep_adjustment_enabled: bool,
    /// None = follow OS local timezone. Some(iana_name) anchors the reset
    /// weekday/time in an explicit IANA zone (e.g. "America/New_York").
    pub tz: Option<String>,
}

impl Default for ResetConfig {
    fn default() -> Self {
        Self {
            weekday: 6, // Sunday
            hour: 0,    // midnight
            minute: 0,
            sleep_start_hour: 23,
            sleep_start_minute: 0,
            sleep_end_hour: 7,
            sleep_end_minute: 0,
            sleep_adjustment_enabled: false,
            tz: None,
        }
    }
}

impl ResetConfig {
    pub fn weekday_name(&self) -> &'static str {
        match self.weekday {
            0 => "Monday",
            1 => "Tuesday",
            2 => "Wednesday",
            3 => "Thursday",
            4 => "Friday",
            5 => "Saturday",
            _ => "Sunday",
        }
    }

    /// Resolve the configured timezone: `Local` if `tz` is unset or unparseable,
    /// otherwise a concrete `chrono_tz::Tz`. An empty string and literal "auto"
    /// both resolve to `Local`.
    pub fn resolve_tz(&self) -> ConfiguredTz {
        match self.tz.as_deref() {
            None | Some("") | Some("auto") => ConfiguredTz::Local,
            Some(name) => name
                .parse::<ChronoTz>()
                .map(ConfiguredTz::Fixed)
                .unwrap_or(ConfiguredTz::Local),
        }
    }

    /// Human-readable timezone label for tooltips / UI display.
    pub fn tz_display_name(&self) -> String {
        match self.resolve_tz() {
            ConfiguredTz::Local => "Local".to_string(),
            ConfiguredTz::Fixed(tz) => tz.name().to_string(),
        }
    }
}

/// Resolved timezone: either the OS local zone or a concrete IANA zone.
pub enum ConfiguredTz {
    Local,
    Fixed(ChronoTz),
}

// ── High-level wrappers (dispatch on the resolved tz) ────────────────────────

/// `(suggested_pct, elapsed_active_secs, week_active_secs)` evaluated at
/// `Utc::now()` under the configured timezone.
pub fn current_usage(cfg: &ResetConfig) -> (f64, f64, f64) {
    match cfg.resolve_tz() {
        ConfiguredTz::Local => suggested_usage(Local::now(), cfg),
        ConfiguredTz::Fixed(tz) => suggested_usage(Utc::now().with_timezone(&tz), cfg),
    }
}

/// Human-readable time-until-next-reset at `Utc::now()` in the configured tz.
pub fn current_time_until_reset(cfg: &ResetConfig) -> String {
    match cfg.resolve_tz() {
        ConfiguredTz::Local => time_until_next_reset(Local::now(), cfg),
        ConfiguredTz::Fixed(tz) => time_until_next_reset(Utc::now().with_timezone(&tz), cfg),
    }
}

/// Simulate the gauge at an arbitrary UTC instant (Unix seconds). Useful for
/// the look-ahead simulator tab.
pub fn simulate_at_unix(unix_secs: i64, cfg: &ResetConfig) -> (f64, f64, f64) {
    let utc = Utc.timestamp_opt(unix_secs, 0).single().unwrap_or_else(Utc::now);
    match cfg.resolve_tz() {
        ConfiguredTz::Local => suggested_usage(utc.with_timezone(&Local), cfg),
        ConfiguredTz::Fixed(tz) => suggested_usage(utc.with_timezone(&tz), cfg),
    }
}

/// Unix timestamp of the next reset in the configured timezone.
pub fn next_reset_unix(cfg: &ResetConfig) -> i64 {
    match cfg.resolve_tz() {
        ConfiguredTz::Local => {
            let now = Local::now();
            let next = last_reset_before(now, cfg) + Duration::weeks(1);
            next.timestamp()
        }
        ConfiguredTz::Fixed(tz) => {
            let now = Utc::now().with_timezone(&tz);
            let next = last_reset_before(now, cfg) + Duration::weeks(1);
            next.timestamp()
        }
    }
}

// ── Generic math ─────────────────────────────────────────────────────────────

/// Returns the most recent reset `DateTime<Tz>` prior to (or equal to) `now`.
pub fn last_reset_before<Tz>(now: DateTime<Tz>, cfg: &ResetConfig) -> DateTime<Tz>
where
    Tz: TimeZone,
    Tz::Offset: Copy,
{
    let reset_time = NaiveTime::from_hms_opt(cfg.hour, cfg.minute, 0).unwrap();
    let target_weekday = weekday_from_u32(cfg.weekday);
    let tz = now.timezone();

    // Walk back up to 7 days to find the last occurrence of (weekday, time) <= now
    for days_back in 0..=7i64 {
        let candidate_date = (now - Duration::days(days_back)).date_naive();
        if candidate_date.weekday() == target_weekday {
            let naive = candidate_date.and_time(reset_time);
            if let Some(dt) = tz.from_local_datetime(&naive).single() {
                if dt <= now {
                    return dt;
                }
            }
        }
    }
    // Fallback: should never happen
    now - Duration::weeks(1)
}

/// Returns `(suggested_pct, elapsed_active_secs, week_active_secs)`.
///
/// When `sleep_adjustment_enabled`, both numerator and denominator are measured in
/// "awake seconds": the gauge does not advance during the daily sleep window, and
/// the full week's total is reduced by seven daily sleep windows. With this math
/// the gauge still reaches ~100% exactly at the true weekly reset.
pub fn suggested_usage<Tz>(now: DateTime<Tz>, cfg: &ResetConfig) -> (f64, f64, f64)
where
    Tz: TimeZone,
    Tz::Offset: Copy,
{
    let last_reset = last_reset_before(now, cfg);
    let raw_elapsed = (now - last_reset).num_seconds().max(0) as f64;
    let raw_week_secs = 7.0 * 24.0 * 3600.0_f64;

    let (elapsed_active, week_active_secs) = if cfg.sleep_adjustment_enabled {
        let start = NaiveTime::from_hms_opt(cfg.sleep_start_hour, cfg.sleep_start_minute, 0)
            .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 0, 0).unwrap());
        let end = NaiveTime::from_hms_opt(cfg.sleep_end_hour, cfg.sleep_end_minute, 0)
            .unwrap_or_else(|| NaiveTime::from_hms_opt(7, 0, 0).unwrap());
        let sleep_in_elapsed = sleep_seconds_in_interval(last_reset, now, start, end);
        let daily_sleep = daily_sleep_duration_secs(start, end);
        let week_sleep = 7.0 * daily_sleep;
        (
            (raw_elapsed - sleep_in_elapsed).max(0.0),
            (raw_week_secs - week_sleep).max(1.0),
        )
    } else {
        (raw_elapsed, raw_week_secs)
    };

    let pct = (elapsed_active / week_active_secs * 100.0).clamp(0.0, 100.0);
    (pct, elapsed_active, week_active_secs)
}

/// Duration of the daily sleep window in seconds. `start == end` means no sleep;
/// otherwise if `end < start` the window wraps past midnight.
pub fn daily_sleep_duration_secs(start: NaiveTime, end: NaiveTime) -> f64 {
    if start == end {
        return 0.0;
    }
    let s = start.num_seconds_from_midnight() as i64;
    let e = end.num_seconds_from_midnight() as i64;
    let diff = if e > s { e - s } else { e + 86_400 - s };
    diff as f64
}

/// Total seconds of overlap between the interval `[from, to]` and the daily sleep
/// window `[start, end)` (possibly wrapping past midnight). Walks day-by-day so the
/// cost is bounded by the number of days in the interval (≤ 8 for a weekly gauge).
pub fn sleep_seconds_in_interval<Tz>(
    from: DateTime<Tz>,
    to: DateTime<Tz>,
    start: NaiveTime,
    end: NaiveTime,
) -> f64
where
    Tz: TimeZone,
    Tz::Offset: Copy,
{
    if to <= from || start == end {
        return 0.0;
    }
    let wraps = end < start;
    let mut total: i64 = 0;
    let tz = from.timezone();

    // Cover from (from.date - 1) through to.date so we catch sleep windows that
    // began the previous calendar day and spill into [from, to].
    let first_day = from.date_naive() - Duration::days(1);
    let last_day = to.date_naive();
    let mut day = first_day;
    while day <= last_day {
        // One sleep occurrence anchored at `day`:
        //   non-wrapping: [day@start, day@end)
        //   wrapping:     [day@start, (day+1)@end)
        let sleep_start = match tz.from_local_datetime(&day.and_time(start)).single() {
            Some(dt) => dt,
            None => {
                day += Duration::days(1);
                continue;
            }
        };
        let sleep_end_date = if wraps { day + Duration::days(1) } else { day };
        let sleep_end = match tz.from_local_datetime(&sleep_end_date.and_time(end)).single() {
            Some(dt) => dt,
            None => {
                day += Duration::days(1);
                continue;
            }
        };

        let lo = if sleep_start > from { sleep_start } else { from };
        let hi = if sleep_end < to { sleep_end } else { to };
        if hi > lo {
            total += (hi - lo).num_seconds();
        }
        day += Duration::days(1);
    }

    total.max(0) as f64
}

/// Human-readable time-until-next-reset string, e.g. "2d 3h 45m".
pub fn time_until_next_reset<Tz>(now: DateTime<Tz>, cfg: &ResetConfig) -> String
where
    Tz: TimeZone,
    Tz::Offset: Copy,
{
    let last_reset = last_reset_before(now, cfg);
    let next_reset = last_reset + Duration::weeks(1);
    let remaining = next_reset - now;

    if remaining.num_seconds() <= 0 {
        return "Resetting now".to_string();
    }

    let total_secs = remaining.num_seconds();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn weekday_from_u32(n: u32) -> Weekday {
    match n {
        0 => Weekday::Mon,
        1 => Weekday::Tue,
        2 => Weekday::Wed,
        3 => Weekday::Thu,
        4 => Weekday::Fri,
        5 => Weekday::Sat,
        _ => Weekday::Sun,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    fn cfg_thursday_7pm() -> ResetConfig {
        ResetConfig { weekday: 3, hour: 19, minute: 0, ..Default::default() }
    }

    /// Verify the example from the spec: Tuesday 7pm (with Thursday 7pm reset)
    /// should give ≈ 71.4%.
    #[test]
    fn test_tuesday_7pm() {
        // 2024-01-04 is a Thursday; 2024-01-09 is the following Tuesday
        let tuesday_7pm = Local.with_ymd_and_hms(2024, 1, 9, 19, 0, 0).unwrap();

        let (pct, _, _) = suggested_usage(tuesday_7pm, &cfg_thursday_7pm());
        // Expected: 5 days elapsed out of 7 → 5/7 * 100 ≈ 71.43%
        assert!((pct - 71.43).abs() < 0.1, "Expected ~71.43%, got {:.2}%", pct);
    }

    /// With sleep adjustment enabled, pct must reach ~100% at the *true* weekly
    /// reset (not mid-week, as the old buggy math produced).
    #[test]
    fn test_sleep_adjustment_end_of_week() {
        let cfg = ResetConfig {
            sleep_adjustment_enabled: true,
            sleep_start_hour: 23,
            sleep_start_minute: 0,
            sleep_end_hour: 7,
            sleep_end_minute: 0,
            ..cfg_thursday_7pm()
        };
        // One second before the next Thursday 7pm reset.
        let almost_next_reset = Local.with_ymd_and_hms(2024, 1, 11, 18, 59, 59).unwrap();
        let (pct, _, _) = suggested_usage(almost_next_reset, &cfg);
        assert!(pct >= 99.5, "Expected ≥ 99.5% at end-of-week, got {:.3}%", pct);
        assert!(pct <= 100.0, "Must be clamped to ≤ 100%, got {:.3}%", pct);
    }

    /// During the sleep window, the gauge must not advance.
    #[test]
    fn test_sleep_adjustment_no_tick_during_sleep() {
        let cfg = ResetConfig {
            sleep_adjustment_enabled: true,
            sleep_start_hour: 23,
            sleep_start_minute: 0,
            sleep_end_hour: 7,
            sleep_end_minute: 0,
            ..cfg_thursday_7pm()
        };
        // Thursday reset at 19:00; sample just before sleep (22:59:59) and inside sleep (03:00 next day).
        let before_sleep = Local.with_ymd_and_hms(2024, 1, 4, 22, 59, 59).unwrap();
        let mid_sleep = Local.with_ymd_and_hms(2024, 1, 5, 3, 0, 0).unwrap();
        let (pct_awake, _, _) = suggested_usage(before_sleep, &cfg);
        let (pct_asleep, _, _) = suggested_usage(mid_sleep, &cfg);
        // They should be essentially identical (within rounding of 1 second slack).
        assert!(
            (pct_asleep - pct_awake).abs() < 0.02,
            "pct should not advance during sleep: before={:.4}% vs mid-sleep={:.4}%",
            pct_awake,
            pct_asleep
        );
    }

    /// A zero-duration sleep window should behave identically to the adjustment being disabled.
    #[test]
    fn test_sleep_adjustment_zero_window() {
        let enabled = ResetConfig {
            sleep_adjustment_enabled: true,
            sleep_start_hour: 0,
            sleep_start_minute: 0,
            sleep_end_hour: 0,
            sleep_end_minute: 0,
            ..cfg_thursday_7pm()
        };
        let disabled = cfg_thursday_7pm();
        let tuesday_7pm = Local.with_ymd_and_hms(2024, 1, 9, 19, 0, 0).unwrap();
        let (pct_enabled, _, _) = suggested_usage(tuesday_7pm, &enabled);
        let (pct_disabled, _, _) = suggested_usage(tuesday_7pm, &disabled);
        assert!(
            (pct_enabled - pct_disabled).abs() < 0.01,
            "Zero-duration window should match disabled: {:.4}% vs {:.4}%",
            pct_enabled,
            pct_disabled
        );
    }

    /// Evaluating the same absolute instant through two timezones whose local
    /// wall-clocks both coincide with the configured reset weekday/time must
    /// yield the same elapsed pct. This guards the "explicit tz override stays
    /// stable across OS tz shifts" guarantee.
    #[test]
    fn test_explicit_tz_stable_across_os_tz() {
        use chrono_tz::{America::New_York, Europe::Berlin};
        // Thursday 2024-01-04 19:00 New_York == Friday 2024-01-05 01:00 Berlin
        // (6h offset). If the user configured reset = Thursday 19:00 in NY,
        // their gauge should read 0% at that instant regardless of whether the
        // app's wall-clock is reporting NY or Berlin.
        let cfg_ny = ResetConfig {
            weekday: 3, hour: 19, minute: 0,
            tz: Some("America/New_York".to_string()),
            ..Default::default()
        };
        let instant_ny = New_York.with_ymd_and_hms(2024, 1, 4, 19, 0, 0).unwrap();
        let (pct_ny, _, _) = suggested_usage(instant_ny, &cfg_ny);
        assert!(pct_ny < 0.01, "At the exact reset instant, pct should be ~0%, got {:.3}", pct_ny);

        // Viewing the same UTC instant through Berlin still picks NY's reset.
        let instant_berlin = instant_ny.with_timezone(&Berlin);
        // current_usage with the same cfg would resolve to NY and compute the
        // same pct. We verify by calling suggested_usage with the NY-viewed dt.
        let (pct_via_ny, _, _) = suggested_usage(instant_berlin.with_timezone(&New_York), &cfg_ny);
        assert!((pct_ny - pct_via_ny).abs() < 0.001,
            "Cross-zone view must match: {:.4}% vs {:.4}%", pct_ny, pct_via_ny);
    }

    /// `simulate_at_unix(now, cfg)` must agree with `current_usage(cfg)` within
    /// the 1-second wall-clock drift between the two calls.
    #[test]
    fn test_simulate_at_matches_current() {
        let cfg = cfg_thursday_7pm();
        let now_unix = Utc::now().timestamp();
        let (pct_sim, _, _) = simulate_at_unix(now_unix, &cfg);
        let (pct_now, _, _) = current_usage(&cfg);
        // Allow 1s of drift (two Utc::now() calls can straddle a second boundary)
        assert!((pct_sim - pct_now).abs() < 0.01,
            "simulate_at must match current within drift: {:.4}% vs {:.4}%", pct_sim, pct_now);
    }

    /// Unknown / unset tz falls back to Local.
    #[test]
    fn test_resolve_tz_fallback() {
        let cfg_none = ResetConfig::default();
        assert!(matches!(cfg_none.resolve_tz(), ConfiguredTz::Local));

        let cfg_auto = ResetConfig { tz: Some("auto".to_string()), ..Default::default() };
        assert!(matches!(cfg_auto.resolve_tz(), ConfiguredTz::Local));

        let cfg_bogus = ResetConfig { tz: Some("Not/A/Zone".to_string()), ..Default::default() };
        assert!(matches!(cfg_bogus.resolve_tz(), ConfiguredTz::Local));

        let cfg_ny = ResetConfig { tz: Some("America/New_York".to_string()), ..Default::default() };
        assert!(matches!(cfg_ny.resolve_tz(), ConfiguredTz::Fixed(_)));
    }
}
