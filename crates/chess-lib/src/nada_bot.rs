use crate::{Chess, ChessExt, ContractError, GAS_FOR_IS_HUMAN_CALL};
use near_sdk::{env, ext_contract, near_bindgen, require, AccountId, Promise};

#[ext_contract(ext_registry)]
pub trait ExtRegistry {
    fn is_human(&self, account_id: AccountId) -> bool;
}

#[near_bindgen]
impl Chess {
    pub fn update_is_human(&mut self, account_id: AccountId) -> Promise {
        require!(self.is_running, "Contract is paused");
        ext_registry::ext(self.nada_bot_id.clone())
            .with_static_gas(GAS_FOR_IS_HUMAN_CALL)
            .is_human(account_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_unused_gas_weight(1)
                    .on_update_is_human(account_id),
            )
    }

    #[private]
    #[handle_result]
    pub fn on_update_is_human(
        &mut self,
        account_id: AccountId,
        #[callback_unwrap] is_human: bool,
    ) -> Result<(), ContractError> {
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        account.set_is_human(is_human);

        Ok(())
    }
}
