pub mod call;
pub mod macros;
pub mod view;

use chess_common::{ContractEvent, KNOWN_EVENT_KINDS};
use chess_lib::{ChessEvent, GameId, GameOutcome};
use near_contract_standards::fungible_token::events::FtMint;
use near_workspaces::{
    network::Sandbox,
    result::{ExecutionFinalResult, ExecutionResult, Value, ViewResultDetails},
    types::{KeyType, SecretKey},
    Account, Contract, Worker,
};
use owo_colors::OwoColorize;
use serde::Serialize;
use serde_json::json;
use std::fmt;
use tokio::fs;

#[macro_export]
macro_rules! print_log {
    ( $x:expr, $($y:expr),+ ) => {
        let thread_name = std::thread::current().name().unwrap().to_string();
        if thread_name == "main" {
            println!($x, $($y),+);
        } else {
            let mut s = format!($x, $($y),+);
            s = s.split('\n').map(|s| {
                let mut pre = "    ".to_string();
                pre.push_str(s);
                pre.push('\n');
                pre
            }).collect::<String>();
            println!(
                "{}\n{}",
                thread_name.bold(),
                &s[..s.len() - 1],
            );
        }
    };
}

pub async fn initialize_contracts(
    path: Option<&'static str>,
) -> anyhow::Result<(Worker<Sandbox>, Account, Contract)> {
    let worker = near_workspaces::sandbox().await?;

    let owner = worker.dev_create_account().await?;

    let wasm = fs::read(path.unwrap_or("../../res/chess_testing.wasm")).await?;

    let key = SecretKey::from_random(KeyType::ED25519);
    let contract = worker
        .create_tla_and_deploy("chess.registrar".parse()?, key, &wasm)
        .await?
        .into_result()?;

    contract
        .call("new")
        .args_json((owner.id(),))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    Ok((worker, owner, contract))
}

pub async fn initialize_token(
    worker: &Worker<Sandbox>,
    name: &str,
    ticker: &str,
    icon: Option<&str>,
    decimals: u8,
) -> anyhow::Result<Contract> {
    let token_contract = worker
        .dev_deploy(&fs::read("../../res/test_token.wasm").await?)
        .await?;
    token_contract
        .call("new")
        .args_json((name, ticker, icon, decimals))
        .transact()
        .await?
        .into_result()?;

    Ok(token_contract)
}

pub fn log_tx_result(
    ident: Option<&str>,
    res: ExecutionFinalResult,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    for failure in res.receipt_failures() {
        print_log!("{:#?}", failure.bright_red());
    }
    let mut events = vec![];
    for outcome in res.receipt_outcomes() {
        if !outcome.logs.is_empty() {
            for log in outcome.logs.iter() {
                if log.starts_with("EVENT_JSON:") {
                    if let Ok(event) =
                        serde_json::from_str::<ContractEvent>(&log.replace("EVENT_JSON:", ""))
                    {
                        events.push(event.clone());
                        print_log!(
                            "{}: {}\n{}",
                            "account".bright_cyan(),
                            outcome.executor_id,
                            event
                        );
                    }
                } else {
                    print_log!("{}", log.bright_yellow());
                }
            }
        }
    }
    if let Some(ident) = ident {
        print_log!(
            "{} gas burnt: {:.3} {}",
            ident.italic(),
            res.total_gas_burnt.as_tgas().bright_magenta().bold(),
            "TGas".bright_magenta().bold()
        );
    }
    Ok((res.into_result()?, events))
}

pub fn log_view_result(res: ViewResultDetails) -> anyhow::Result<ViewResultDetails> {
    if !res.logs.is_empty() {
        for log in res.logs.iter() {
            print_log!("{}", log.bright_yellow());
        }
    }
    Ok(res)
}

pub fn assert_event_emits<T>(actual: T, events: Vec<ChessEvent>) -> anyhow::Result<()>
where
    T: Serialize + fmt::Debug + Clone,
{
    let mut actual = serde_json::to_value(&actual)?;
    actual.as_array_mut().unwrap().retain(|ac| {
        let event_str = ac
            .as_object()
            .unwrap()
            .get("event")
            .unwrap()
            .as_str()
            .unwrap();
        KNOWN_EVENT_KINDS.contains(&event_str)
    });
    let mut expected = vec![];
    for event in events {
        let mut expected_event = serde_json::to_value(event)?;
        let ev = expected_event.as_object_mut().unwrap();
        let event_str = ev.get("event").unwrap().as_str().unwrap();
        if !KNOWN_EVENT_KINDS.contains(&event_str) {
            continue;
        }
        ev.insert("standard".into(), "chess-game".into());
        expected.push(expected_event);
    }
    assert_eq!(
        &actual,
        &serde_json::to_value(&expected)?,
        "actual and expected events did not match.\nActual: {:#?}\nExpected: {:#?}",
        &actual,
        &expected
    );
    Ok(())
}

pub fn assert_ft_mint_events<T>(actual: T, events: Vec<FtMint>) -> anyhow::Result<()>
where
    T: Serialize + fmt::Debug + Clone,
{
    let mut actual = serde_json::to_value(&actual)?;
    actual.as_array_mut().unwrap().retain(|ac| {
        let event_str = ac
            .as_object()
            .unwrap()
            .get("event")
            .unwrap()
            .as_str()
            .unwrap();
        event_str == "ft_mint"
    });
    let mut expected = vec![];
    for event in events {
        expected.push(json!({
            "event": "ft_mint",
            "standard": "nep141",
            "version": "1.0.0",
            "data": [event]
        }));
    }
    assert_eq!(
        &actual,
        &serde_json::to_value(&expected)?,
        "actual and expected events did not match.\nActual: {:#?}\nExpected: {:#?}",
        &actual,
        &expected
    );
    Ok(())
}

const STALEMATE_MOVES: [&str; 18] = [
    "e2e3", "a7a5", "d1h5", "a8a6", "h5a5", "h7h5", "a5c7", "a6h6", "h2h4", "f7f6", "c7d7",
    "e8f7", "d7b7", "d8d3", "b7b8", "d3h7", "b8c8", "f7g6",
];
const STALEMATE_LAST_MOVE: &str = "c8e6";

pub async fn play_stalemate_moves_except_last(
    contract: &Contract,
    white: &Account,
    black: &Account,
    game_id: &GameId,
) -> anyhow::Result<()> {
    for (i, mv) in STALEMATE_MOVES.iter().enumerate() {
        let player = if i % 2 == 0 { white } else { black };
        call::play_move(contract, player, game_id, mv.to_string()).await?;
    }
    Ok(())
}

pub async fn play_stalemate_game(
    contract: &Contract,
    white: &Account,
    black: &Account,
    game_id: &GameId,
) -> anyhow::Result<GameOutcome> {
    play_stalemate_moves_except_last(contract, white, black, game_id).await?;
    let ((outcome, _), _, _) =
        call::play_move(contract, white, game_id, STALEMATE_LAST_MOVE.to_string()).await?;
    let outcome = outcome.unwrap();
    assert_eq!(outcome, GameOutcome::Stalemate);
    Ok(outcome)
}

pub fn get_game_id(events: &[ContractEvent]) -> GameId {
    use chess_common::{AcceptChallenge, ChessEvent as ChessEventCommon, ChessEventKind};
    events
        .iter()
        .find_map(|event| {
            if let ContractEvent::ChessGame(ChessEventCommon {
                event_kind: ChessEventKind::AcceptChallenge(AcceptChallenge { game_id, .. }),
                ..
            }) = event
            {
                Some(game_id)
            } else {
                None
            }
        })
        .unwrap()
        .clone()
}
