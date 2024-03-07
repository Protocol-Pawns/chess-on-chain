pub mod call;
pub mod macros;
pub mod view;

use chess_common::{ContractEvent, KNOWN_EVENT_KINDS};
use chess_lib::{chess_notification_to_value, ChessEvent, ChessNotification};
use near_contract_standards::fungible_token::events::FtMint;
use near_sdk::AccountId;
use near_workspaces::{
    network::Sandbox,
    result::{ExecutionFinalResult, ExecutionResult, Value, ViewResultDetails},
    types::{KeyType, NearToken, SecretKey},
    Account, Contract, Worker,
};
use owo_colors::OwoColorize;
use serde::Serialize;
use serde_json::json;
use std::{collections::HashMap, fmt};
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
) -> anyhow::Result<(Worker<Sandbox>, Account, Contract, Contract, Contract)> {
    let worker = near_workspaces::sandbox().await?;

    let owner = worker.dev_create_account().await?;

    let wasm = fs::read(path.unwrap_or("../../res/chess_testing.wasm")).await?;

    let key = SecretKey::from_random(KeyType::ED25519);
    let social_contract = worker
        .create_tla_and_deploy(
            "social.test.near".parse()?,
            key,
            &fs::read("../../res/social_db.wasm").await?,
        )
        .await?
        .into_result()?;
    social_contract
        .call("new")
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    social_contract
        .call("set_status")
        .args_json(("Live",))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let key = SecretKey::from_random(KeyType::ED25519);
    let nada_bot_contract = worker
        .create_tla_and_deploy(
            "nada-bot.test.near".parse()?,
            key,
            &fs::read("../../res/nada_bot_stub.wasm").await?,
        )
        .await?
        .into_result()?;
    nada_bot_contract
        .call("new")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let key = SecretKey::from_random(KeyType::ED25519);
    let contract = worker
        .create_tla_and_deploy("chess.test.near".parse()?, key, &wasm)
        .await?
        .into_result()?;

    contract
        .call("new")
        .args_json((social_contract.id(), nada_bot_contract.id()))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    contract
        .as_account()
        .call(social_contract.id(), "set")
        .args_json(serde_json::json!({
            "data": {
                contract.id().as_str(): {}
            }
        }))
        .deposit(NearToken::from_near(2))
        .transact()
        .await?
        .into_result()?;

    Ok((worker, owner, contract, social_contract, nada_bot_contract))
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

pub async fn assert_notification(
    contract: &Contract,
    social_contract: &Contract,
    notifications: HashMap<AccountId, Vec<ChessNotification>>,
) -> anyhow::Result<()> {
    let actual_notification = view::get_social(
        social_contract,
        vec![format!("{}/index/notify", contract.id()).to_string()],
    )
    .await?;
    let mut actual: serde_json::Value = serde_json::from_str(
        actual_notification
            .get(contract.id())
            .unwrap()
            .get("index")
            .unwrap()
            .get("notify")
            .unwrap()
            .as_str()
            .unwrap(),
    )?;
    let sorting = |a: &serde_json::Value, b: &serde_json::Value| {
        a.get("key")
            .unwrap()
            .as_str()
            .unwrap()
            .cmp(b.get("key").unwrap().as_str().unwrap())
    };
    actual.as_array_mut().unwrap().sort_by(sorting);
    let mut expected = json!(&notifications
        .iter()
        .flat_map(|(account_id, notifications)| {
            let mut notifications: Vec<_> = notifications
                .iter()
                .map(|notification| chess_notification_to_value(account_id, notification))
                .collect();
            notifications.push(json!({
                "key": account_id,
                "value": {
                    "type": "poke"
                }
            }));
            notifications
        })
        .collect::<Vec<_>>());
    expected.as_array_mut().unwrap().sort_by(sorting);
    assert_eq!(actual, expected);
    Ok(())
}
