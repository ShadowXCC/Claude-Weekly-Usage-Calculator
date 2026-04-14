/// Core usage math.
///
/// The Claude usage week is defined as a 7-day rolling window that resets
/// at a user-specified day-of-week and time. The "suggested usage %" at any
/// moment is simply:
///
///   elapsed_seconds / (7 * 24 * 3600) * 100.0
///
/// where elapsed_seconds is the number of seconds since the most recent reset.
/// Sub-minute precision is maintained throughout.

use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

/// Day of week (0 = Monday … 6 = Sunday), matching chrono's num_days_from_monday.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ResetConfig {
    /// 0 = Monday, 1 = Tuesday, … 6 = Sunday
    pub weekday: u32,
    pub hour: u32,
    pub minute: u32,
}

impl Default for ResetConfig {
    fn default() -> Self {
        Self {
            weekday: 6, // Sunday
            hour: 0,    // midnight
            minute: 0,
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
}

/// Returns the most recent reset `DateTime<Local>` prior to (or equal to) `now`.
pub fn last_reset_before(now: DateTime<Local>, cfg: &ResetConfig) -> DateTime<Local> {
    let reset_time = NaiveTime::from_hms_opt(cfg.hour, cfg.minute, 0).unwrap();
    let target_weekday = weekday_from_u32(cfg.weekday);

    // Walk back up to 7 days to find the last occurrence of (weekday, time) <= now
    for days_back in 0..=7i64 {
        let candidate_date = (now - Duration::days(days_back)).date_naive();
        if candidate_date.weekday() == target_weekday {
            let candidate = candidate_date
                .and_time(reset_time)
                .and_local_timezone(Local)
                .single();
            if let Some(dt) = candidate {
                if dt <= now {
                    return dt;
                }
            }
        }
    }
    // Fallback: should never happen
    now - Duration::weeks(1)
}

/// Returns `(suggested_pct, elapsed_secs, total_week_secs)` given the current time and config.
pub fn suggested_usage(now: DateTime<Local>, cfg: &ResetConfig) -> (f64, f64, f64) {
    let total_week_secs = 7.0 * 24.0 * 3600.0_f64;
    let last_reset = last_reset_before(now, cfg);
    let elapsed = (now - last_reset).num_seconds().max(0) as f64;
    let elapsed = elapsed.min(total_week_secs);
    let pct = elapsed / total_week_secs * 100.0;
    (pct, elapsed, total_week_secs)
}

/// Human-readable time-until-next-reset string, e.g. "2d 3h 45m".
pub fn time_until_next_reset(now: DateTime<Local>, cfg: &ResetConfig) -> String {
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

    /// Verify the example from the spec: Tuesday 7pm (with Thursday 7pm reset)
    /// should give ≈ 71.4%.
    #[test]
    fn test_tuesday_7pm() {
        // Find a Thursday 7pm in the past, then compute Tuesday 7pm of the same week
        let cfg = ResetConfig { weekday: 3, hour: 19, minute: 0 };

        // 2024-01-04 is a Thursday; 2024-01-09 is the following Tuesday
        let tuesday_7pm = Local
            .with_ymd_and_hms(2024, 1, 9, 19, 0, 0)
            .unwrap();

        let (pct, elapsed, total) = suggested_usage(tuesday_7pm, &cfg);
        // Expected: 5 days elapsed out of 7 → 5/7 * 100 ≈ 71.43%
        assert!((pct - 71.43).abs() < 0.1, "Expected ~71.43%, got {:.2}%", pct);
    }
}
