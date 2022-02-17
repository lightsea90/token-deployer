use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenAllocation {
    pub allocated_percent: u64,
    pub initial_release: u64,
    pub vesting_start_time: Timestamp,
    pub vesting_end_time: Timestamp,
    pub vesting_interval: Duration,
    pub claimed: u64,
}

impl Default for TokenAllocation {
    fn default() -> Self {
        Self {
            allocated_percent: 0,
            initial_release: 0,
            vesting_start_time: 0,
            vesting_end_time: 0,
            vesting_interval: 0,
            claimed: 1,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct WrappedTokenAllocation {
    pub allocated_percent: u64,
    pub initial_release: u64,
    pub vesting_start_time: WrappedTimestamp,
    pub vesting_end_time: WrappedTimestamp,
    pub vesting_interval: WrappedDuration,
}

impl From<TokenAllocation> for WrappedTokenAllocation {
    fn from(allocs: TokenAllocation) -> Self {
        WrappedTokenAllocation {
            allocated_percent: allocs.allocated_percent,
            initial_release: allocs.initial_release,
            vesting_start_time: WrappedTimestamp::from(allocs.vesting_start_time),
            vesting_end_time: WrappedTimestamp::from(allocs.vesting_end_time),
            vesting_interval: WrappedDuration::from(allocs.vesting_interval)
        }

    }
}

pub type TokenAllocationInput = HashMap<AccountId, WrappedTokenAllocation>;
