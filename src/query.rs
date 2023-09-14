use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractConfig)]
    Config {},
    #[returns(QueryMembersResponse)]
    Members {},
}

#[cw_serde]
pub struct QueryMembersResponse {
    pub members: Vec<Addr>,
}