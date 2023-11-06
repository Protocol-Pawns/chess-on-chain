use super::{event, log_tx_result};
use chess_lib::{
    AcceptChallengeMsg, ChallengeId, ChallengeMsg, Difficulty, FtReceiverMsg, GameId, GameOutcome,
    MoveStr,
};
use near_sdk::json_types::U128;
use near_workspaces::{
    result::{ExecutionFinalResult, ExecutionResult, Value},
    types::Balance,
    Account, AccountId, Contract,
};
use serde::Serialize;
use serde_json::json;

pub async fn migrate(
    contract: &Contract,
    sender: &Account,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _): (ExecutionResult<Value>, Vec<event::ContractEvent>) = log_tx_result(
        Some("migrate"),
        sender
            .call(contract.id(), "migrate")
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn add_human(
    contract: &Contract,
    sender: &Account,
    account_id: &AccountId,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("add_human"),
        sender
            .call(contract.id(), "add_human")
            .args_json((account_id,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn update_is_human(
    contract: &Contract,
    sender: &Account,
    account_id: &AccountId,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("update_is_human"),
        sender
            .call(contract.id(), "update_is_human")
            .args_json((account_id,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn storage_deposit(
    contract: &Contract,
    sender: &Account,
    account_id: Option<&AccountId>,
    deposit: Option<Balance>,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("storage_deposit"),
        sender
            .call(contract.id(), "storage_deposit")
            .args_json((account_id, None::<bool>))
            .deposit(deposit.unwrap_or(50_000_000_000_000_000_000_000))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn mint_tokens(
    token: &Contract,
    receiver: &AccountId,
    amount: u128,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        None,
        token
            .call("mint")
            .args_json((receiver, U128::from(amount)))
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn create_ai_game(
    contract: &Contract,
    sender: &Account,
    difficulty: Difficulty,
) -> anyhow::Result<(GameId, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<event::ContractEvent>) = log_tx_result(
        Some("create_ai_game"),
        sender
            .call(contract.id(), "create_ai_game")
            .args_json((difficulty,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res.json()?, events))
}

pub async fn create_ai_game_old(
    contract: &Contract,
    sender: &Account,
    difficulty: Difficulty,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _): (ExecutionResult<Value>, Vec<event::ContractEvent>) = log_tx_result(
        Some("create_ai_game_old"),
        sender
            .call(contract.id(), "create_ai_game")
            .args_json((difficulty,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn challenge(
    contract: &Contract,
    sender: &Account,
    challenged_id: &AccountId,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<event::ContractEvent>) = log_tx_result(
        Some("challenge"),
        sender
            .call(contract.id(), "challenge")
            .args_json((challenged_id,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res, events))
}

pub async fn challenge_with_wager(
    sender: &Account,
    token_id: &AccountId,
    receiver_id: &AccountId,
    amount: U128,
    msg: ChallengeMsg,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<event::ContractEvent>) = log_tx_result(
        Some("challenge_with_wager"),
        ft_transfer_call(
            sender,
            token_id,
            receiver_id,
            amount,
            FtReceiverMsg::Challenge(msg),
        )
        .await?,
    )?;
    Ok((res, events))
}

pub async fn accept_challenge(
    contract: &Contract,
    sender: &Account,
    challenge_id: &ChallengeId,
) -> anyhow::Result<(GameId, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<event::ContractEvent>) = log_tx_result(
        Some("accept_challenge"),
        sender
            .call(contract.id(), "accept_challenge")
            .args_json((challenge_id,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res.json()?, events))
}

pub async fn accept_challenge_with_wager(
    sender: &Account,
    token_id: &AccountId,
    receiver_id: &AccountId,
    amount: U128,
    msg: AcceptChallengeMsg,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<event::ContractEvent>) = log_tx_result(
        Some("accept_challenge_with_wager"),
        ft_transfer_call(
            sender,
            token_id,
            receiver_id,
            amount,
            FtReceiverMsg::AcceptChallenge(msg),
        )
        .await?,
    )?;
    Ok((res, events))
}

pub async fn reject_challenge(
    contract: &Contract,
    sender: &Account,
    challenge_id: &ChallengeId,
    is_challenger: bool,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<event::ContractEvent>) = log_tx_result(
        Some("reject_challenge"),
        sender
            .call(contract.id(), "reject_challenge")
            .args_json((challenge_id, is_challenger))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res, events))
}

pub async fn play_move(
    contract: &Contract,
    sender: &Account,
    game_id: &GameId,
    mv: MoveStr,
) -> anyhow::Result<(
    (Option<GameOutcome>, [String; 8]),
    Vec<event::ContractEvent>,
)> {
    let (res, events) = log_tx_result(
        Some("play_move"),
        sender
            .call(contract.id(), "play_move")
            .args_json((game_id, mv))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res.json()?, events))
}

pub async fn resign(
    contract: &Contract,
    sender: &Account,
    game_id: &GameId,
) -> anyhow::Result<(GameOutcome, Vec<event::ContractEvent>)> {
    let (res, events) = log_tx_result(
        Some("resign"),
        sender
            .call(contract.id(), "resign")
            .args_json((game_id,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res.json()?, events))
}

pub async fn cancel(
    contract: &Contract,
    sender: &Account,
    game_id: &GameId,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<event::ContractEvent>)> {
    let (res, events) = log_tx_result(
        Some("cancel"),
        sender
            .call(contract.id(), "cancel")
            .args_json((game_id,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res, events))
}

async fn ft_transfer_call<T: Serialize>(
    sender: &Account,
    token_id: &AccountId,
    receiver_id: &AccountId,
    amount: U128,
    msg: T,
) -> anyhow::Result<ExecutionFinalResult> {
    Ok(sender
        .call(token_id, "ft_transfer_call")
        .args_json((
            receiver_id,
            amount,
            Option::<String>::None,
            json!(msg).to_string(),
        ))
        .max_gas()
        .deposit(1)
        .transact()
        .await?)
}
