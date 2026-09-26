#[tokio::main]
async fn main() -> anyhow::Result<()> {
    data_notification::run_notification_consumer().await
}
