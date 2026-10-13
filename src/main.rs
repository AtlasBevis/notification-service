#[tokio::main]
async fn main() -> data_notification::AppResult<()> {
    data_notification::run_api().await
}
