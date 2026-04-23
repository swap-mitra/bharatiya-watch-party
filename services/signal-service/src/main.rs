#[tokio::main]
async fn main() -> Result<(), app_core::AppError> {
    signal_service::run().await
}
