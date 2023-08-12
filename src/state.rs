use cw_storage_plus::{Map, Item};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Timestamp, Addr};

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
	pub vesting_span: u64,
	// token address
	pub vesting_token_addr: Addr,
}

pub const SHAREHOLDERS: Map<&Addr, ShareholderInfo> = Map::new("shareholders");
pub const CONFIG: Item<ContractConfig> = Item::new("config");
