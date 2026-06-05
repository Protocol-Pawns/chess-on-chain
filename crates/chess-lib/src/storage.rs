use crate::{Chess, ChessExt, ContractError, NO_DEPOSIT};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{assert_one_yocto, env, near_bindgen, require, AccountId, NearToken, Promise};

pub const STORAGE_ACCOUNT_COST: NearToken = NearToken::from_millinear(50);

#[near_bindgen]
impl StorageManagement for Chess {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        require!(self.is_running, "Contract is paused");
        match self.internal_storage_deposit(account_id, registration_only) {
            Ok(res) => res,
            Err(err) => {
                panic!("{}", err);
            }
        }
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        require!(self.is_running, "Contract is paused");
        assert_one_yocto();
        match self.internal_storage_withdraw(amount) {
            Ok(res) => res,
            Err(err) => {
                panic!("{}", err);
            }
        }
    }

    #[payable]
    fn storage_unregister(&mut self, _force: Option<bool>) -> bool {
        panic!("storage_unregister is not implemented");
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: STORAGE_ACCOUNT_COST,
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_get_account(&account_id)
            .ok()
            .map(|account| StorageBalance {
                total: account.get_near_amount(),
                available: NO_DEPOSIT,
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
        let min_balance = self.storage_balance_bounds().min;
        let already_registered = self.accounts.contains_key(&account_id);
        if amount < min_balance && !already_registered {
            return Err(ContractError::NotEnoughDeposit(
                min_balance.as_yoctonear(),
                amount.as_yoctonear(),
            ));
        }
        if already_registered {
            if amount.as_yoctonear() > 0 {
                let _ = Promise::new(env::predecessor_account_id()).transfer(amount);
            }
            Ok(self.storage_balance_of(account_id).unwrap())
        } else {
            self.internal_register_account(account_id.clone(), min_balance);
            let refund = amount.checked_sub(min_balance).unwrap();
            if refund.as_yoctonear() > 0 {
                let _ = Promise::new(env::predecessor_account_id()).transfer(refund);
            }
            Ok(StorageBalance {
                total: min_balance,
                available: NO_DEPOSIT,
            })
        }
    }

    fn internal_storage_withdraw(
        &mut self,
        _amount: Option<NearToken>,
    ) -> Result<StorageBalance, ContractError> {
        Err(ContractError::OperationNotSupported)
    }
}
