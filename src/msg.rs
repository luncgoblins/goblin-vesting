use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InitialShareholder {
	pub addr: String,
	pub weight: u64,
}

#[cw_serde]
pub struct InstantiateMsg {
	pub vesting_span: u64,
	pub token: String,
	pub shareholders: Vec<InitialShareholder>,
}

#[cw_serde]
pub enum ExecuteMsg {
	Withdraw {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
