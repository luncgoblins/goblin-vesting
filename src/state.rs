use crate::msg::AssetInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::{Item, Map};

//ShareholderInfo holds shareholder info and state
#[cw_serde]
pub struct ShareholderInfo {
    // last withdraw block height
    pub last_withdraw_timestamp: Timestamp,
    // shareholders weight over a common denominator
    pub weight: u64,
}

// ContractInfo holds global contract state
#[cw_serde]
pub struct ContractConfig {
    // total length of vesting span in seconds
    pub vesting_period: u64,
    pub vesting_amount: u64,
    // token address
    pub vesting_token: AssetInfo,
    pub admin: Addr,
    pub schedule_start: Timestamp,
    pub force_withdraw_enabled: bool,
}

pub const SHAREHOLDERS: Map<&Addr, ShareholderInfo> = Map::new("shareholders");
pub const CONFIG: Item<ContractConfig> = Item::new("config");
