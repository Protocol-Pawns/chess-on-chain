mod indexer;
mod send;

pub use indexer::*;
pub use send::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let stream = indexer::start_indexing().await?;
    send_data(stream).await?;

    Ok(())
}
