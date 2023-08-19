use cosmwasm_schema::write_api;

use goblin_vesting::msg::{ExecuteMsg, InstantiateMsg};
use goblin_vesting::query::QueryMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
