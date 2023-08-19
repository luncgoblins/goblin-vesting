use cosmwasm_schema::{cw_serde, QueryResponses};
use crate::state::{ContractConfig};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractConfig)]
    Config {},
}
