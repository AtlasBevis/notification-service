#[tokio::main]
async fn main() -> data_notification::error::AppResult<()> {
    data_notification::run_notification_consumer().await
}
