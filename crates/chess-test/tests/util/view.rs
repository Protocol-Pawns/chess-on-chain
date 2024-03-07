use super::log_view_result;
use chess_lib::{
    AccountInfo, Achievement, BetInfo, Challenge, ChallengeId, Fees, GameId, GameInfo, Quest,
};
use near_sdk::json_types::U128;
use near_workspaces::{AccountId, Contract};
use serde_json::Value;
use std::collections::HashMap;

pub async fn get_game_ids(
    contract: &Contract,
    account_id: &AccountId,
) -> anyhow::Result<Vec<GameId>> {
    let res = log_view_result(
        contract
            .call("get_game_ids")
            .args_json((account_id,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_game_info(contract: &Contract, game_id: &GameId) -> anyhow::Result<GameInfo> {
    let res = log_view_result(
        contract
            .call("game_info")
            .args_json((game_id,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_bet_info(
    contract: &Contract,
    players: (&AccountId, &AccountId),
) -> anyhow::Result<BetInfo> {
    let res = log_view_result(
        contract
            .call("bet_info")
            .args_json((players,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_account(
    contract: &Contract,
    account_id: &AccountId,
) -> anyhow::Result<AccountInfo> {
    let res = log_view_result(
        contract
            .call("get_account")
            .args_json((account_id,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_quest_cooldowns(
    contract: &Contract,
    account_id: &AccountId,
) -> anyhow::Result<Vec<(u64, Quest)>> {
    let res = log_view_result(
        contract
            .call("get_quest_cooldowns")
            .args_json((account_id,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_achievements(
    contract: &Contract,
    account_id: &AccountId,
) -> anyhow::Result<Vec<(u64, Achievement)>> {
    let res = log_view_result(
        contract
            .call("get_achievements")
            .args_json((account_id,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_challenges(
    contract: &Contract,
    account_id: &AccountId,
    is_challenger: bool,
) -> anyhow::Result<Vec<ChallengeId>> {
    let res = log_view_result(
        contract
            .call("get_challenges")
            .args_json((account_id, is_challenger))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_challenge(
    contract: &Contract,
    challenge_id: &ChallengeId,
) -> anyhow::Result<Challenge> {
    let res = log_view_result(
        contract
            .call("get_challenge")
            .args_json((challenge_id,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_fees(contract: &Contract) -> anyhow::Result<Fees> {
    let res = log_view_result(contract.call("get_fees").max_gas().view().await?)?;
    Ok(res.json()?)
}

pub async fn get_token_whitelist(contract: &Contract) -> anyhow::Result<Vec<AccountId>> {
    let res = log_view_result(
        contract
            .call("get_token_whitelist")
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_social(
    contract: &Contract,
    keys: Vec<String>,
) -> anyhow::Result<HashMap<AccountId, Value>> {
    let res = log_view_result(
        contract
            .call("get")
            .args_json((keys, None::<String>))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn ft_balance_of(contract: &Contract, account_id: &AccountId) -> anyhow::Result<U128> {
    let res = log_view_result(
        contract
            .call("ft_balance_of")
            .args_json((account_id,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn ft_total_supply(contract: &Contract) -> anyhow::Result<U128> {
    let res = log_view_result(contract.call("ft_total_supply").max_gas().view().await?)?;
    Ok(res.json()?)
}
