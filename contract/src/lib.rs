/*
Functions:

 */


// To conserve gas, efficient serialization is achieved through Borsh (http://borsh.io/)
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::serde_json::{json, Value};
use near_sdk::{env, near_bindgen, ext_contract, PanicOnDefault};
use near_sdk::collections::{UnorderedMap};
use near_sdk::{AccountId, Balance, Timestamp, Duration, Gas};
use near_sdk::{Promise, PromiseResult};
use near_sdk::json_types::{WrappedBalance, WrappedDuration, WrappedTimestamp};
use std::collections::HashMap;

near_sdk::setup_alloc!();

const DEFAULT_GAS_FEE: Gas = 20_000_000_000_000;
const TOKEN_FACTORY_ACCOUNT: &str = "token-factory.tokenhub.testnet";

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenAllocation {
    allocated_num: Balance,
    initial_release: Balance,
    vesting_start_time: Timestamp,
    vesting_end_time: Timestamp,
    vesting_interval: Duration,
    claimed: Balance,
}

impl Default for TokenAllocation {
    fn default() -> Self {
        Self {
            allocated_num: 0,
            initial_release: 0,
            vesting_start_time: 0,
            vesting_end_time: 0,
            vesting_interval: 0,
            claimed: 1,
        }
    }
}

#[ext_contract(ext_self)]
pub trait ExtTokenAllocation {
    fn on_claim_finished(&mut self, predecessor_account_id: AccountId, amount: Balance) -> bool;
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct WrappedTokenAllocation {
    allocated_num: WrappedBalance,
    initial_release: WrappedBalance,
    vesting_start_time: WrappedTimestamp,
    vesting_end_time: WrappedTimestamp,
    vesting_interval: WrappedDuration,
}

pub type TokenAllocationInput = HashMap<AccountId, WrappedTokenAllocation>;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct TokenDeployer {
    ft_contract_name: AccountId,
    total_supply: Balance,
    allocations: UnorderedMap<AccountId, TokenAllocation>,
}

#[near_bindgen]
impl TokenDeployer {

    #[init]
    pub fn new(
        ft_contract_name: String,
        total_supply: WrappedBalance,
        allocations: TokenAllocationInput,
    ) -> Self {
        assert!(
            !env::state_exists(),
            "The contract is already initialized",
        );
        assert!(
            env::predecessor_account_id() == TOKEN_FACTORY_ACCOUNT,
            "Only token factory is allowed to execute the function"
        );

        let mut s = Self {
            ft_contract_name: ft_contract_name,
            total_supply: total_supply.into(),
            allocations: UnorderedMap::new(b"alloc".to_vec()),
        };
        for (account_id, alloc) in &allocations {
            let a = TokenAllocation {
                allocated_num: alloc.allocated_num.into(),
                initial_release: alloc.initial_release.into(),
                vesting_start_time: alloc.vesting_start_time.into(),
                vesting_end_time: alloc.vesting_end_time.into(),
                vesting_interval: alloc.vesting_interval.into(),
                claimed: 0,
            };
            assert!(
                a.allocated_num >= a.initial_release + a.claimed,
                "Allocation is smaller than the total claimable",
            );
            assert!(
                a.vesting_interval <= a.vesting_end_time - a.vesting_start_time,
                "Vesting interval is larger than vesting time",
            );
            s.allocations.insert(account_id, &a);
        }
        return s;
    }

    // fn validate_allocation_list(self) {
    //     check if total allocation <= total supply
    // }

    pub fn get_allocation_list(self) -> Value {
        json!({})
    }

    fn get_claimable_amount(&self, alloc: &TokenAllocation) -> Balance {
        let currrent_ts = env::block_timestamp();
        let claimable_num: Balance = {
            if currrent_ts < alloc.vesting_start_time {
                0
            } else if currrent_ts >= alloc.vesting_end_time {
                alloc.allocated_num - alloc.initial_release
            }
            else {
                let intervals: u64 = 
                    (currrent_ts - alloc.vesting_start_time) / alloc.vesting_interval;
                let total_intervals: u64 =
                    (alloc.vesting_end_time - alloc.vesting_start_time) / alloc.vesting_interval;
                
                // result
                (alloc.allocated_num - alloc.initial_release)
                    / total_intervals as Balance * intervals as Balance
            }
        };
        let claimable_amount: Balance = claimable_num + alloc.initial_release - alloc.claimed;
        return claimable_amount;
    }

    pub fn check_account(&self, account_id: AccountId) -> Value {
        let alloc = self.allocations.get(&account_id).unwrap_or_default();
        self.validate_alloc(&alloc);
        let claimable_amount: Balance = self.get_claimable_amount(&alloc);

        return json!({
            "allocated_num": WrappedBalance::from(alloc.allocated_num),
            "initial_release": WrappedBalance::from(alloc.initial_release),
            "vesting_start_time": WrappedTimestamp::from(alloc.vesting_start_time),
            "vesting_end_time": WrappedTimestamp::from(alloc.vesting_end_time),
            "vesting_interval": WrappedDuration::from(alloc.vesting_interval),
            "claimed": WrappedBalance::from(alloc.claimed),
            "claimable_amount": WrappedBalance::from(claimable_amount),
        });
    }

    pub fn claim(&mut self) -> Promise {
        let account_id = env::signer_account_id();
        let alloc = self.allocations.get(&account_id).unwrap_or_default();
        let amount_to_claim: Balance = self.get_claimable_amount(&alloc);
        env::log(
            format!("amount to claim = {}", amount_to_claim)
            .as_bytes()
        );
        assert!(
            amount_to_claim > 0,
            "There is nothing to claim at the moment",
        );
        let transfer_promise = Promise::new(self.ft_contract_name.clone()).function_call(
            b"ft_transfer".to_vec(), 
            json!({
                "receiver_id": account_id,
                "amount": WrappedBalance::from(amount_to_claim),
            }).to_string().as_bytes().to_vec(), 
            1, DEFAULT_GAS_FEE,
        );

        return transfer_promise.then(
            ext_self::on_claim_finished(
                account_id, amount_to_claim,
                &env::current_account_id(),
                0, DEFAULT_GAS_FEE,
            )
        );
    }

    #[private]
    pub fn on_claim_finished(
        &mut self,
        predecessor_account_id: AccountId,
        amount: Balance
    ) -> bool {
        assert!(
            env::promise_results_count() == 1,
            "Function called not as a callback",
        );
        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                let mut alloc = self.allocations.remove(&predecessor_account_id).unwrap_or_default();
                self.validate_alloc(&alloc);
                assert!(
                    alloc.claimed + amount <= alloc.allocated_num,
                    "Something wrong. Total claimed is greater than allocated_num",
                );
                alloc.claimed += amount;
                self.allocations.insert(&predecessor_account_id, &alloc);
                true
            },
            _ => false
        }
    }

    fn validate_alloc(&self, alloc: &TokenAllocation) -> bool {
        assert!(
            alloc.vesting_end_time > 0,
            "Not a valid allocation",
        );
        assert!(
            alloc.vesting_end_time > alloc.vesting_start_time,
            "vesting_end_time is smaller than vesting_start_time",
        );
        return true;
    }

}

/*
 * The rest of this file holds the inline tests for the code above
 * Learn more about Rust tests: https://doc.rust-lang.org/book/ch11-01-writing-tests.html
 *
 * To run from contract directory:
 * cargo test -- --nocapture
 *
 * From project root, to run in combination with frontend tests:
 * yarn test
 *
 */

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    // mock the context for testing, notice "signer_account_id" that was accessed above from env::
    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "tokensale_near".to_string(),
            signer_account_id: "harrynguyen_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "harrynguyen_near".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 1_000_000_000_000_000_000_000_000,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
            epoch_height: 19,
        }
    }


}