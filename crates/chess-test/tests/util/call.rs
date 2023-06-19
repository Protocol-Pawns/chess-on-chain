use super::{event, log_tx_result};
use chess_lib::{ChallengeId, Difficulty, GameId, GameOutcome, MoveStr};
use workspaces::{
    result::{ExecutionResult, Value},
    types::Balance,
    Account, AccountId, Contract,
};

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

pub async fn create_ai_game(
    contract: &Contract,
    sender: &Account,
    difficulty: Difficulty,
) -> anyhow::Result<(GameId, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<super::event::ContractEvent>) = log_tx_result(
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

pub async fn challenge(
    contract: &Contract,
    sender: &Account,
    challenged_id: &AccountId,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<super::event::ContractEvent>) = log_tx_result(
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

pub async fn accept_challenge(
    contract: &Contract,
    sender: &Account,
    challenge_id: ChallengeId,
) -> anyhow::Result<(GameId, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<super::event::ContractEvent>) = log_tx_result(
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

pub async fn reject_challenge(
    contract: &Contract,
    sender: &Account,
    challenge_id: ChallengeId,
    is_challenger: bool,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<event::ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<super::event::ContractEvent>) = log_tx_result(
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
