use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};

use crate::error::{AppError, AppResult};

pub struct TimeUtils;

impl TimeUtils {
    /// `2026-10-14 10:19:14`
    pub const FMT_DATETIME: &'static str = "%Y-%m-%d %H:%M:%S";
    /// `20261014101914`
    pub const FMT_DATETIME_COMPACT: &'static str = "%Y%m%d%H%M%S";
    /// `2026-10-14`
    pub const FMT_DATE: &'static str = "%Y-%m-%d";
    /// `2026-10-14T10:19:14+07:00`
    pub const FMT_ISO8601_OFFSET: &'static str = "%Y-%m-%dT%H:%M:%S%:z";
    /// `20261014`, the `date_key` used in templates
    pub const FMT_DATE_COMPACT: &'static str = "%Y%m%d";

    /// Vietnam time (`+07:00`).
    pub const VN_OFFSET_HOURS: i32 = 7;

    pub fn now() -> DateTime<Utc> {
        Utc::now()
    }

    /// Formats in UTC.
    pub fn format(time: &DateTime<Utc>, fmt: &str) -> String {
        time.format(fmt).to_string()
    }

    /// Formats in the given UTC offset, e.g. `7` for Vietnam time.
    pub fn format_at(time: &DateTime<Utc>, offset_hours: i32, fmt: &str) -> AppResult<String> {
        let offset = FixedOffset::east_opt(offset_hours * 3600).ok_or_else(|| {
            AppError::bad_request(format!("invalid UTC offset `{offset_hours}` hours"))
        })?;
        Ok(time.with_timezone(&offset).format(fmt).to_string())
    }

    /// Formats in Vietnam time (`+07:00`).
    pub fn format_vn(time: &DateTime<Utc>, fmt: &str) -> String {
        let offset = FixedOffset::east_opt(Self::VN_OFFSET_HOURS * 3600)
            .expect("+07:00 is a valid UTC offset");
        time.with_timezone(&offset).format(fmt).to_string()
    }

    /// Parses `value` laid out as `fmt`. Formats with an offset (`%z`) keep it;
    /// others are read as UTC, and formats without a time part give midnight.
    pub fn parse(value: &str, fmt: &str) -> AppResult<DateTime<Utc>> {
        let value = value.trim();
        let invalid =
            || AppError::bad_request(format!("`{value}` does not match time format `{fmt}`"));

        if fmt.contains("%z") || fmt.contains("%:z") {
            return DateTime::parse_from_str(value, fmt)
                .map(|time| time.with_timezone(&Utc))
                .map_err(|_| invalid());
        }

        let naive = if fmt.contains("%H") {
            NaiveDateTime::parse_from_str(value, fmt).ok()
        } else {
            NaiveDate::parse_from_str(value, fmt)
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
        };
        naive
            .map(|time| Utc.from_utc_datetime(&time))
            .ok_or_else(invalid)
    }
}
