use chrono::{FixedOffset, Utc};
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct VietnamTime;

impl FormatTime for VietnamTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let vn = FixedOffset::east_opt(7 * 3600).expect("UTC+7");
        let now = Utc::now().with_timezone(&vn);
        write!(
            w,
            "{}.{:03}{}",
            now.format("%Y-%m-%dT%H:%M:%S"),
            now.timestamp_subsec_millis(),
            now.format("%:z"),
        )
    }
}

/// Install the process-wide JSON logger (UTC+7 timestamps). Call once at startup.
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "data_notification=info,tower_http=info".into()),
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(false)
                .with_timer(VietnamTime),
        )
        .init();
}
