use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFormat {
    /// `2026-10-14T10:19:14+07:00`
    Iso8601Offset,
    /// `2026-10-14 10:19:14`
    DateTime,
    /// `2026-10-14`
    Date,
    /// `10:19:14`
    Time,
    /// `20261014`, the `date_key` used in templates
    DateKey,
    /// `20261014101914`
    Compact,
}

impl TimeFormat {
    pub fn pattern(self) -> &'static str {
        match self {
            Self::Iso8601Offset => "%Y-%m-%dT%H:%M:%S%:z",
            Self::DateTime => "%Y-%m-%d %H:%M:%S",
            Self::Date => "%Y-%m-%d",
            Self::Time => "%H:%M:%S",
            Self::DateKey => "%Y%m%d",
            Self::Compact => "%Y%m%d%H%M%S",
        }
    }

    /// Formats without a time part parse as midnight.
    fn is_date_only(self) -> bool {
        matches!(self, Self::Date | Self::DateKey)
    }
}

pub struct TimeUtils;

impl TimeUtils {
    /// Vietnam time (`+07:00`).
    pub const VN_OFFSET_HOURS: i32 = 7;

    pub fn now() -> DateTime<Utc> {
        Utc::now()
    }

    /// Formats in UTC.
    pub fn format(time: &DateTime<Utc>, format: TimeFormat) -> String {
        time.format(format.pattern()).to_string()
    }

    /// Formats in the given UTC offset, e.g. `7` for Vietnam time.
    pub fn format_at(
        time: &DateTime<Utc>,
        offset_hours: i32,
        format: TimeFormat,
    ) -> AppResult<String> {
        let offset = FixedOffset::east_opt(offset_hours * 3600).ok_or_else(|| {
            AppError::bad_request(format!("invalid UTC offset `{offset_hours}` hours"))
        })?;
        Ok(time
            .with_timezone(&offset)
            .format(format.pattern())
            .to_string())
    }

    /// Formats in Vietnam time (`+07:00`).
    pub fn format_vn(time: &DateTime<Utc>, format: TimeFormat) -> String {
        let offset = FixedOffset::east_opt(Self::VN_OFFSET_HOURS * 3600)
            .expect("+07:00 is a valid UTC offset");
        time.with_timezone(&offset)
            .format(format.pattern())
            .to_string()
    }

    /// Parses `value` laid out as `format`. Values without an offset are read
    /// as UTC; date-only formats give midnight.
    pub fn parse(value: &str, format: TimeFormat) -> AppResult<DateTime<Utc>> {
        let value = value.trim();
        let invalid = || {
            AppError::bad_request(format!(
                "`{value}` does not match time format `{}`",
                format.pattern()
            ))
        };

        if format == TimeFormat::Iso8601Offset {
            return DateTime::parse_from_str(value, format.pattern())
                .map(|time| time.with_timezone(&Utc))
                .map_err(|_| invalid());
        }
        if format == TimeFormat::Time {
            return Err(AppError::bad_request(
                "a time without a date cannot be parsed to a point in time",
            ));
        }

        let naive = if format.is_date_only() {
            NaiveDate::parse_from_str(value, format.pattern())
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
        } else {
            NaiveDateTime::parse_from_str(value, format.pattern()).ok()
        };
        naive
            .map(|time| Utc.from_utc_datetime(&time))
            .ok_or_else(invalid)
    }
}
