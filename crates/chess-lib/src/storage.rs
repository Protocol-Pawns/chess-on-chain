use crate::{Chess, ChessExt, ContractError};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{
    assert_one_yocto, env, json_types::U128, near_bindgen, AccountId, Balance, Promise,
};

/// 0.1N
pub const STORAGE_ACCOUNT_COST: Balance = 100_000_000_000_000_000_000_000;

#[near_bindgen]
impl StorageManagement for Chess {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        match self.internal_storage_deposit(account_id, registration_only) {
            Ok(res) => res,
            Err(err) => {
                panic!("{}", err);
            }
        }
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();
        match self.internal_storage_withdraw(amount) {
            Ok(res) => res,
            Err(err) => {
                panic!("{}", err);
            }
        }
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();
        match self.internal_storage_unregister(force) {
            Ok(res) => res,
            Err(err) => {
                panic!("{}", err);
            }
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: STORAGE_ACCOUNT_COST.into(),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_get_account(&account_id)
            .ok()
            .map(|account| StorageBalance {
                total: U128(account.near_amount),
                available: U128(0),
            })
    }
}

impl Chess {
    fn internal_storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        _registration_only: Option<bool>,
    ) -> Result<StorageBalance, ContractError> {
        let amount = env::attached_deposit();
        let account_id = account_id.unwrap_or_else(env::predecessor_account_id);
        let min_balance = self.storage_balance_bounds().min.0;
        let already_registered = self.accounts.contains_key(&account_id);
        if amount < min_balance && !already_registered {
            return Err(ContractError::NotEnoughDeposit(min_balance, amount));
        }
        if already_registered {
            if amount > 0 {
                Promise::new(env::predecessor_account_id()).transfer(amount);
            }
        } else {
            self.internal_register_account(account_id.clone(), min_balance);
            let refund = amount - min_balance;
            if refund > 0 {
                Promise::new(env::predecessor_account_id()).transfer(refund);
            }
        }
        Ok(self.storage_balance_of(account_id).unwrap())
    }

    fn internal_storage_withdraw(
        &mut self,
        _amount: Option<U128>,
    ) -> Result<StorageBalance, ContractError> {
        Err(ContractError::OperationNotSupported)
    }

    fn internal_storage_unregister(&mut self, force: Option<bool>) -> Result<bool, ContractError> {
        if force.is_some() {
            return Err(ContractError::OperationNotSupported);
        }
        let account_id = env::predecessor_account_id();
        if let Ok(account) = self.internal_get_account(&account_id) {
            if account.is_playing() {
                return Err(ContractError::AccountIsPlaying);
            }
            Promise::new(account_id.clone()).transfer(account.near_amount);
            self.accounts.remove(&account_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
