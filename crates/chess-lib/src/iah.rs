use crate::{Chess, ChessExt, ContractError, GAS_FOR_IS_HUMAN_CALL};
use near_sdk::{env, ext_contract, near_bindgen, AccountId, Promise};

#[ext_contract(ext_registry)]
pub trait ExtRegistry {
    fn is_human(&self, account: AccountId) -> Vec<(AccountId, Vec<u64>)>;
}

#[near_bindgen]
impl Chess {
    pub fn update_is_human(&mut self, account_id: AccountId) -> Promise {
        ext_registry::ext(self.iah_registry.clone())
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
        #[callback_unwrap] is_human: Vec<(AccountId, Vec<u64>)>,
    ) -> Result<(), ContractError> {
        let account = self
            .accounts
            .get_mut(&account_id)
            .ok_or_else(|| ContractError::AccountNotRegistered(account_id.clone()))?;
        account.set_is_human(!is_human.is_empty());

        Ok(())
    }
}
