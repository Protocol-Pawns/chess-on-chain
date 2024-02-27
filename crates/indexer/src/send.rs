use anyhow::Result;
use chess_common::ChessEvent;
use futures_util::pin_mut;
use near_lake_framework::near_indexer_primitives::types::BlockHeight;
use reqwest::{
    header::{HeaderMap, AUTHORIZATION},
    Client, Url,
};
use serde::{Deserialize, Serialize};
use std::env;
use tokio_stream::{Stream, StreamExt};

#[derive(Serialize, Deserialize, Debug)]
struct BatchEvent {
    pub block_height: BlockHeight,
    pub timestamp: u64,
    pub events: Vec<ChessEvent>,
}

pub async fn send_data(
    stream: impl Stream<Item = (BlockHeight, u64, Vec<ChessEvent>)>,
) -> Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {}", env::var("INDEXER_SECRET")?).parse()?,
    );
    let client = Client::builder().default_headers(headers).build()?;
    let base_url = Url::parse(&env::var("API_BASE_URL")?)?;
    pin_mut!(stream);

    let mut last_block_height = 0;
    const MAX_BLOCK_HEIGHT_DIFF: BlockHeight = 100;
    while let Some((block_height, timestamp, events)) = stream.next().await {
        let batch_event = BatchEvent {
            block_height,
            timestamp,
            events,
        };

        if !batch_event.events.is_empty()
            || block_height - last_block_height >= MAX_BLOCK_HEIGHT_DIFF
        {
            println!("block_height: {}", block_height);
            last_block_height = block_height;
            match client
                .post(base_url.join("batch")?)
                .json(&batch_event)
                .send()
                .await?
                .error_for_status()
            {
                Ok(_) => {}
                Err(err) => {
                    panic!(
                        "{}\n\nSent data:\n{:#?}",
                        err,
                        serde_json::to_value(batch_event)
                    );
                }
            }
        }
    }

    Ok(())
}
