use super::log_tx_result;
use chess_common::ContractEvent;
use chess_lib::{
    AcceptChallengeMsg, BetMsg, ChallengeId, ChallengeMsg, Difficulty, FtReceiverMsg, GameId,
    GameOutcome, MatchmakingMsg, MoveStr,
};
use near_sdk::{json_types::U128, Gas};
use near_workspaces::{
    result::{ExecutionFinalResult, ExecutionResult, Value},
    types::NearToken,
    Account, AccountId, Contract, CryptoHash,
};
use serde::Serialize;
use serde_json::json;

pub async fn migrate(
    contract: &Contract,
    sender: &Account,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
        Some("migrate"),
        sender
            .call(contract.id(), "migrate")
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn set_is_agent_no_deposit(
    contract: &Contract,
    sender: &Account,
    is_agent: bool,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("set_is_agent_no_deposit"),
        sender
            .call(contract.id(), "set_is_agent")
            .args_json((is_agent,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn set_is_agent(
    contract: &Contract,
    sender: &Account,
    is_agent: bool,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("set_is_agent"),
        sender
            .call(contract.id(), "set_is_agent")
            .args_json((is_agent,))
            .deposit(NearToken::from_yoctonear(1))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn claim_points(
    contract: &Contract,
    sender: &Account,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("claim_points"),
        sender
            .call(contract.id(), "claim_points")
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn pause(
    contract: &Contract,
    sender: &Account,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("pause"),
        sender
            .call(contract.id(), "pause")
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn resume(
    contract: &Contract,
    sender: &Account,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("resume"),
        sender
            .call(contract.id(), "resume")
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok(res)
}

pub async fn set_fees(
    contract: &Contract,
    sender: &Account,
    treasury: u16,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
        Some("set_fees"),
        sender
            .call(contract.id(), "set_fees")
            .args_json(serde_json::json!({ "treasury": treasury }))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res, events))
}

pub async fn set_token_whitelist(
    contract: &Contract,
    sender: &Account,
    whitelist: &[AccountId],
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
        Some("set_token_whitelist"),
        sender
            .call(contract.id(), "set_token_whitelist")
            .args_json((whitelist,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res, events))
}

pub async fn storage_deposit(
    contract: &Contract,
    sender: &Account,
    account_id: Option<&AccountId>,
    deposit: Option<NearToken>,
) -> anyhow::Result<ExecutionResult<Value>> {
    let (res, _) = log_tx_result(
        Some("storage_deposit"),
        sender
            .call(contract.id(), "storage_deposit")
            .args_json((account_id, None::<bool>))
            .deposit(deposit.unwrap_or(NearToken::from_millinear(50)))
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
) -> anyhow::Result<(GameId, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
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
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
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
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
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
) -> anyhow::Result<(GameId, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
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
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
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
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
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

pub async fn join_matchmaking(
    contract: &Contract,
    sender: &Account,
    min_elo: f64,
    max_elo: f64,
) -> anyhow::Result<(Option<GameId>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
        Some("join_matchmaking"),
        sender
            .call(contract.id(), "join_matchmaking")
            .args_json((min_elo, max_elo))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res.json()?, events))
}

pub async fn join_matchmaking_with_wager(
    sender: &Account,
    token_id: &AccountId,
    receiver_id: &AccountId,
    amount: U128,
    msg: MatchmakingMsg,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
        Some("join_matchmaking_with_wager"),
        ft_transfer_call(
            sender,
            token_id,
            receiver_id,
            amount,
            FtReceiverMsg::Matchmaking(msg),
        )
        .await?,
    )?;
    Ok((res, events))
}

pub async fn cancel_matchmaking(
    contract: &Contract,
    sender: &Account,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
        Some("cancel_matchmaking"),
        sender
            .call(contract.id(), "cancel_matchmaking")
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
    CryptoHash,
    Vec<ContractEvent>,
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
    Ok((res.json()?, res.receipt_outcomes()[0].block_hash, events))
}

pub async fn play_move_raw(
    contract: &Contract,
    sender: &Account,
    game_id: &GameId,
    mv: MoveStr,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    log_tx_result(
        Some("play_move"),
        sender
            .call(contract.id(), "play_move")
            .args_json((game_id, mv))
            // .max_gas()
            .gas(Gas::from_tgas(1000))
            .transact()
            .await?,
    )
}

pub async fn resign(
    contract: &Contract,
    sender: &Account,
    game_id: &GameId,
) -> anyhow::Result<(GameOutcome, Vec<ContractEvent>)> {
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
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
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

pub async fn cancel_bet(
    contract: &Contract,
    sender: &Account,
    players: (AccountId, AccountId),
    token_id: &AccountId,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events) = log_tx_result(
        Some("cancel_bet"),
        sender
            .call(contract.id(), "cancel_bet")
            .args_json((&players, &token_id))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res, events))
}

pub async fn bet(
    sender: &Account,
    token_id: &AccountId,
    receiver_id: &AccountId,
    amount: U128,
    msg: BetMsg,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events): (ExecutionResult<Value>, Vec<ContractEvent>) = log_tx_result(
        Some("bet"),
        ft_transfer_call(
            sender,
            token_id,
            receiver_id,
            amount,
            FtReceiverMsg::Bet(msg),
        )
        .await?,
    )?;
    Ok((res, events))
}

pub async fn withdraw_treasury(
    contract: &Contract,
    sender: &Account,
    token_id: &AccountId,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events) = log_tx_result(
        Some("withdraw_treasury"),
        sender
            .call(contract.id(), "withdraw_treasury")
            .args_json((token_id,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res, events))
}

pub async fn transfer_ownership(
    contract: &Contract,
    sender: &Account,
    new_owner_id: &AccountId,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events) = log_tx_result(
        Some("transfer_ownership"),
        sender
            .call(contract.id(), "transfer_ownership")
            .args_json((new_owner_id,))
            .max_gas()
            .transact()
            .await?,
    )?;
    Ok((res, events))
}

pub async fn withdraw_token(
    contract: &Contract,
    sender: &Account,
    token_id: &AccountId,
) -> anyhow::Result<(ExecutionResult<Value>, Vec<ContractEvent>)> {
    let (res, events) = log_tx_result(
        Some("withdraw_token"),
        sender
            .call(contract.id(), "withdraw_token")
            .args_json((token_id,))
            .deposit(NearToken::from_yoctonear(1))
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
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?)
}
