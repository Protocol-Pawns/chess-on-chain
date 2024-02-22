use anyhow::Result;
use async_stream::stream;
use chess_common::{ChessEvent, ChessEventKind, ContractEvent};
use futures_core::Stream;
use near_lake_framework::{
    near_indexer_primitives::{
        types::{AccountId, BlockHeight},
        views::{
            ExecutionOutcomeView, ExecutionOutcomeWithIdView, ExecutionStatusView, ReceiptEnumView,
            ReceiptView,
        },
        IndexerExecutionOutcomeWithReceipt, StreamerMessage,
    },
    LakeConfigBuilder,
};
use once_cell::sync::Lazy;
use owo_colors::OwoColorize;
use reqwest::{
    header::{HeaderMap, AUTHORIZATION},
    Client, Url,
};
use serde::Deserialize;
use std::env;

static GAME_ACCOUNT_ID: Lazy<AccountId> =
    Lazy::new(|| env::var("GAME_ACCOUNT_ID").unwrap().parse().unwrap());

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Info {
    last_block_height: u64,
}

pub async fn start_indexing() -> Result<impl Stream<Item = (BlockHeight, u64, Vec<ChessEvent>)>> {
    let start_block_height = get_current_block_height().await?;

    let config = LakeConfigBuilder::default()
        .mainnet()
        .start_block_height(start_block_height)
        .build()
        .unwrap();

    let (_, mut stream) = near_lake_framework::streamer(config);

    Ok(stream! {
        while let Some(msg) = stream.recv().await {
            let block_height = msg.block.header.height;
            let timestamp = msg.block.header.timestamp_nanosec;
            let events = handle_message(msg);

            yield (block_height, timestamp, events);
        }
    })
}

fn handle_message(msg: StreamerMessage) -> Vec<ChessEvent> {
    let mut res = vec![];
    for shard in msg.shards {
        for IndexerExecutionOutcomeWithReceipt {
            execution_outcome: ExecutionOutcomeWithIdView { outcome, .. },
            receipt:
                ReceiptView {
                    receipt,
                    receiver_id,
                    ..
                },
        } in shard.receipt_execution_outcomes
        {
            if receiver_id != *GAME_ACCOUNT_ID {
                continue;
            }
            match outcome.status {
                ExecutionStatusView::Unknown | ExecutionStatusView::Failure(_) => continue,
                _ => {}
            }

            if let ReceiptEnumView::Action { .. } = receipt {
                let mut events = extract_events(msg.block.header.timestamp / 1_000_000, &outcome);
                res.append(&mut events);
            }
        }

        if let Some(chunk) = shard.chunk {
            for transaction in chunk.transactions {
                if transaction.transaction.receiver_id != *GAME_ACCOUNT_ID {
                    continue;
                }
                match transaction.outcome.execution_outcome.outcome.status {
                    ExecutionStatusView::Unknown | ExecutionStatusView::Failure(_) => continue,
                    _ => {}
                }
                let mut events = extract_events(
                    msg.block.header.timestamp_nanosec / 1_000_000,
                    &transaction.outcome.execution_outcome.outcome,
                );
                res.append(&mut events);
            }
        }
    }
    res
}

async fn get_current_block_height() -> anyhow::Result<u64> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {}", env::var("INDEXER_SECRET")?).parse()?,
    );
    let client = Client::builder().default_headers(headers).build()?;
    let base_url = Url::parse(&env::var("INDEXER_API_URL")?)?;
    let info: Info = client
        .get(base_url.join("info")?)
        .send()
        .await?
        .json()
        .await?;
    if info.last_block_height > 0 {
        Ok(info.last_block_height + 1)
    } else {
        Ok(env::var("START_BLOCK_HEIGHT").unwrap().parse()?)
    }
}

fn extract_events(timestamp_ms: u64, outcome: &ExecutionOutcomeView) -> Vec<ChessEvent> {
    let prefix = "EVENT_JSON:";
    outcome
        .logs
        .iter()
        .filter_map(|untrimmed_log| {
            let log = untrimmed_log.trim();
            if !log.starts_with(prefix) {
                return None;
            }

            if let Ok(ContractEvent::ChessGame(event)) =
                serde_json::from_str::<ContractEvent>(log[prefix.len()..].trim())
            {
                match &event.event_kind {
                    ChessEventKind::CreateGame(_)
                    | ChessEventKind::PlayMove(_)
                    | ChessEventKind::ResignGame(_)
                    | ChessEventKind::CancelGame(_) => {
                        println!(
                            "\n{}{}{}\n{}",
                            "=== new event (".bright_yellow(),
                            timestamp_ms.bright_yellow(),
                            ") ===".bright_yellow(),
                            &event
                        );
                        Some(event)
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect()
}
