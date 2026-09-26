#[tokio::main]
async fn main() -> anyhow::Result<()> {
    data_notification::run_api().await
}
