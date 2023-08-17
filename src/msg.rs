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
	pub admin: String,
}

#[cw_serde]
pub enum ExecuteMsg {
	Withdraw {},
	AddMember {
		addr: String,
		weight: u64,
	},
	RemoveMember {
		addr: String,
		compensation: u64,
	},
	KickOff {
		date: u64,
	},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
