#[tokio::main]
async fn main() -> Result<(), api::shared::error::AppError> {
    api::app::run().await
}
