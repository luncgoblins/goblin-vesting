use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InitialShareholder {
    pub addr: String,
    pub weight: u64,
}

#[cw_serde]
pub enum AssetInfo {
    Cw20Info { address: String },
    NativeInfo { denom: String },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub vesting_period: u64,
    pub vesting_amount: u64,
    pub token: AssetInfo,
    pub shareholders: Vec<InitialShareholder>,
    pub admin: String,
    pub force_withdraw_enabled: Option<bool>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Withdraw {},
    AddMember { addr: String, weight: u64 },
    RemoveMember { addr: String, compensation: u64 },
    KickOff { date: u64 },
    ForceWithdraw {},
}
