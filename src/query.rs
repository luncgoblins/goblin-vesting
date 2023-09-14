use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use crate::state::{ContractConfig};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractConfig)]
    Config {},
    #[returns(QueryMembersResponse)]
    Members {},
    #[returns(QueryMemberResponse)]
    Member { addr: Addr },
}

#[cw_serde]
pub struct QueryMembersResponse {
    pub members: Vec<Addr>,
}

#[cw_serde]
pub struct QueryMemberResponse {
    pub addr: Addr,
    pub weight_nominator: u64,
    pub weight_denominator: u64,
    pub can_withdraw: Uint128,
}