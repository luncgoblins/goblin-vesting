use cosmwasm_schema::{cw_serde};

#[cw_serde]
pub struct InitialShareholder {
	pub addr: String,
	pub weight: u64,
}

#[cw_serde]
pub struct InstantiateMsg {
	pub vesting_period: u64,
	pub vesting_amount: u64,
	pub token: String,
	pub shareholders: Vec<InitialShareholder>,
	pub admin: String,
	pub force_withdraw_enabled: Option<bool>,
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
	ForceWithdraw {},
}
