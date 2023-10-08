use super::log_view_result;
use chess_lib::{Challenge, ChallengeId, EloRating, GameId, GameInfo, IndexNotify};
use std::collections::HashMap;
use workspaces::{AccountId, Contract};

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

pub async fn get_elo(contract: &Contract, account_id: &AccountId) -> anyhow::Result<EloRating> {
    let res = log_view_result(
        contract
            .call("get_elo")
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

pub async fn recent_finished_games(contract: &Contract) -> anyhow::Result<Vec<GameId>> {
    let res = log_view_result(
        contract
            .call("recent_finished_games")
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn finished_games(
    contract: &Contract,
    account_id: &AccountId,
) -> anyhow::Result<Vec<GameId>> {
    let res = log_view_result(
        contract
            .call("finished_games")
            .args_json((account_id,))
            .max_gas()
            .view()
            .await?,
    )?;
    Ok(res.json()?)
}

pub async fn get_social(
    contract: &Contract,
    keys: Vec<String>,
) -> anyhow::Result<HashMap<AccountId, IndexNotify>> {
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
