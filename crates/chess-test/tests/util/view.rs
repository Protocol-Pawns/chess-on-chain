use super::log_view_result;
use chess_lib::{ChallengeId, GameId};
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
