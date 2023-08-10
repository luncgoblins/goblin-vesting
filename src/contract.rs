#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, QueryRequest, to_binary, WasmQuery, Uint128, WasmMsg};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{SHAREHOLDERS, CONFIG, ContractConfig, ShareholderInfo};
use cw20::{BalanceResponse, Cw20QueryMsg, Cw20ExecuteMsg};

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:goblin-vesting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    
    let config = ContractConfig{
		vesting_span: msg.vesting_span,
		weight_denominator: msg.denominator,
		vesting_token_addr: deps.api.addr_validate(
			&msg.token
		)?,
	};
	CONFIG.save(deps.storage, &config)?;
	
	let mut sum = 0u64;
	for member in msg.shareholders.iter() {
		let shareholder_info = ShareholderInfo{
			last_withdraw_timestamp: env.block.time,
			weight_nominator: member.nominator,
		};
		let shareholder_addr = deps.api.addr_validate(&member.addr)?;
        SHAREHOLDERS.save(deps.storage, &shareholder_addr, &shareholder_info)?;
        sum += member.nominator;
    }
    
    if sum != config.weight_denominator {
		return Err(ContractError::InitializeError{});
	}

	Ok(Response::new())
    
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
	match msg {
		ExecuteMsg::Withdraw {} => {
			execute_withdraw(deps, env, info)
		}
	}
}

pub fn execute_withdraw(
	deps: DepsMut,
	env: Env,
	info: MessageInfo,
) -> Result<Response, ContractError> {

	if !SHAREHOLDERS.has(deps.storage, &info.sender) {
		return Err(ContractError::Unauthorized{});
	}
	let config = CONFIG.load(deps.storage)?;
	let mut member_info = SHAREHOLDERS.load(deps.storage, &info.sender)?;
	let balance = query_balance(&deps, env.clone(), info.clone())?;
	let weight = (
		Uint128::from(member_info.weight_nominator),
		Uint128::from(config.weight_denominator)
	);
	let time_diff = Uint128::from(
		env.block.time
		.minus_seconds(member_info.last_withdraw_timestamp.seconds()).seconds()
	);
	let time_weight = (
		time_diff,
		Uint128::from(config.vesting_span),
	);
	let eligible_withdraw_amnt = balance
		.checked_mul_floor(time_weight).unwrap_or(Uint128::from(0u64))
		.checked_mul_floor(weight).unwrap_or(Uint128::from(0u64));
	
	// update state
	member_info.last_withdraw_timestamp = env.block.time;
	SHAREHOLDERS.save(deps.storage, &info.sender, &member_info)?;
	
	let send_request = send_tokens(
		&deps, env.clone(), info.clone(),
		eligible_withdraw_amnt,
		info.sender
	)?;
	Ok(Response::new()
		.add_message(send_request)
	)
}

pub fn send_tokens(
	deps: &DepsMut,
	_env: Env,
	_info: MessageInfo,
	amnt: Uint128,
	dst: Addr,
) -> StdResult<WasmMsg> {
	
	let config = CONFIG.load(deps.storage)?;
	let send_msg = Cw20ExecuteMsg::Transfer {
		recipient: dst.into_string(),
		amount: amnt,
	};
	let wasm_msg = WasmMsg::Execute {
		contract_addr: config.vesting_token_addr.into_string(),
        msg: to_binary(&send_msg)?,
        funds: vec![],
	};
	Ok(wasm_msg)
}

pub fn query_balance(
	deps: &DepsMut,
	env: Env,
	_info: MessageInfo,
) -> Result<Uint128, ContractError> {
	
	let query_msg = Cw20QueryMsg::Balance {
		address: env.contract.address.into_string(),
	};
	let config = CONFIG.load(deps.storage)?;
	let request = QueryRequest::Wasm({
		WasmQuery::Smart{
			contract_addr: config.vesting_token_addr.into_string(),
			msg: to_binary(&query_msg)?,
		}
	});
	let response: BalanceResponse = deps.querier.query(&request)?;
	Ok(response.balance)

}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
