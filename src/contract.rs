#[cfg(not(feature = "library"))]

use cosmwasm_std::entry_point;
use cosmwasm_std::{Timestamp,Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, QueryRequest, to_binary, WasmQuery, Uint128, WasmMsg};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::query::{QueryMsg};
use crate::state::{SHAREHOLDERS, CONFIG, ContractConfig, ShareholderInfo};
use cw20::{BalanceResponse, Cw20QueryMsg, Cw20ExecuteMsg};
use cosmwasm_std::Order::Ascending;

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
		vesting_token_addr: deps.api.addr_validate(
			&msg.token
		)?,
		vesting_token_balance: Uint128::from(0u64),
		admin: deps.api.addr_validate(
			&msg.admin
		)?,
		schedule_start: Timestamp::from_seconds(0u64),
	};
	CONFIG.save(deps.storage, &config)?;
	
	for member in msg.shareholders.iter() {
		let shareholder_info = ShareholderInfo{
			last_withdraw_timestamp: env.block.time,
			weight: member.weight,
		};
		let shareholder_addr = deps.api.addr_validate(&member.addr)?;
        SHAREHOLDERS.save(deps.storage, &shareholder_addr, &shareholder_info)?;
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
		},
		ExecuteMsg::AddMember { addr, weight } => {
			let in_addr = deps.api.addr_validate(&addr)?;
			execute_add_member(deps, env, info, &in_addr, weight)
		},
		ExecuteMsg::RemoveMember { addr, compensation } => {
			let in_addr = deps.api.addr_validate(&addr)?;
			execute_remove_member(deps, env, info, &in_addr, compensation)
		},
		ExecuteMsg::KickOff { date } => {
			execute_kickoff(deps, env, info, Timestamp::from_seconds(date))
		},
	}
}

pub fn is_expired(
	start_timestamp: Timestamp,
	current_timestamp: Timestamp,
	duration: Timestamp,
) -> bool {

	current_timestamp > start_timestamp.plus_seconds(duration.seconds())

}

pub fn is_inactive(
	start_timestamp: Timestamp,
	current_timestamp: Timestamp,
	duration: Timestamp,
) -> bool {

	let a = !is_kickstarted(start_timestamp);
	let b = is_kickstarted(start_timestamp) && current_timestamp < start_timestamp.plus_seconds(duration.seconds());
	a || b

}

pub fn is_kickstarted(
	start_timestamp: Timestamp,
) -> bool {

	start_timestamp > Timestamp::from_seconds(0u64)

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
	if is_inactive(
		config.schedule_start,
		env.block.time,
		Timestamp::from_seconds(config.vesting_span),
	) {
		return Err(ContractError::InactiveContract{});
	}

	let addr = &info.sender;
	let curr_time = env.block.time;
	let mut member_info = SHAREHOLDERS.load(deps.storage, &addr)?;
	let withdraw_amnt = calculate_withdraw_amnt(&deps, env.clone(), info.clone(), &addr)?;
	
	// update state
	member_info.last_withdraw_timestamp = curr_time;
	SHAREHOLDERS.save(deps.storage, &addr, &member_info)?;
	
	let send_request = get_withdraw_msg(
		&deps, env.clone(), info.clone(),
		withdraw_amnt,
		&info.sender
	)?;
	Ok(Response::new()
		.add_message(send_request)
	)
}

pub fn execute_remove_member(
	deps: DepsMut,
	env: Env,
	info: MessageInfo,
	addr: &Addr,
	compensation: u64,
) -> Result<Response, ContractError> {

	// only admin can remove members
	let config = CONFIG.load(deps.storage)?;
	if config.admin != info.sender {
		return Err(ContractError::Unauthorized{});
	}
	
	// removed member must be current board member
	if !SHAREHOLDERS.has(deps.storage, &info.sender) {
		return Err(ContractError::UnexpectedInput{});
	}
	
	if is_expired(
		config.schedule_start,
		env.block.time,
		Timestamp::from_seconds(config.vesting_span),
	) {
		return Err(ContractError::ExpiredContract{});
	}

	let curr_timestamp = env.block.time;
	
	// force withdraw for all members
	// (but only if linear payout started)
	let mut msgs = vec![];
	if !is_inactive(
		config.schedule_start,
		env.block.time,
		Timestamp::from_seconds(config.vesting_span),
	) {
		msgs = SHAREHOLDERS
			.range(deps.storage, None, None, Ascending)
			.collect::<StdResult<Vec<_>>>()?
			.iter()
			.map(|item| -> Result<WasmMsg, ContractError>  {
				let withdraw_amnt = calculate_withdraw_amnt(&deps, env.clone(), info.clone(), &item.0)?;
				let message = get_withdraw_msg(&deps, env.clone(), info.clone(), withdraw_amnt, &item.0)?;
				SHAREHOLDERS.update(deps.storage, &item.0, |info: Option<ShareholderInfo>| -> Result<ShareholderInfo, ContractError> {
					let mut ret = info.ok_or(ContractError::UnexpectedInput{})?;
					ret.last_withdraw_timestamp = curr_timestamp;
					Ok(ret)
				})?;
				Ok(message)
			})
			.collect::<Result<Vec<_>, ContractError>>()?;
	}

	// remove member
	SHAREHOLDERS.remove(deps.storage, addr);
	
	// compensation message
	let compensation_msg = get_withdraw_msg(
		&deps, env.clone(), info.clone(),
		Uint128::from(compensation), addr
	)?;
	
	// emit
	Ok(Response::new()
		.add_messages(msgs)
		.add_message(compensation_msg)
	)
}

pub fn execute_add_member(
	deps: DepsMut,
	env: Env,
	info: MessageInfo,
	addr: &Addr,
	weight: u64,
) -> Result<Response, ContractError> {

	// only admin can add members
	let config = CONFIG.load(deps.storage)?;
	if config.admin != info.sender {
		return Err(ContractError::Unauthorized{});
	}
	
	// new member not in list of current members
	if SHAREHOLDERS.has(deps.storage, &info.sender) {
		return Err(ContractError::UnexpectedInput{});
	}
	
	if is_expired(
		config.schedule_start,
		env.block.time,
		Timestamp::from_seconds(config.vesting_span),
	) {
		return Err(ContractError::ExpiredContract{});
	}
	
	let curr_timestamp = env.block.time;
	
	// force withdraw for all members
	// (but only if linear payout started)
	let mut msgs = vec![];
	if !is_inactive(
		config.schedule_start,
		env.block.time,
		Timestamp::from_seconds(config.vesting_span),
	) {
		msgs = SHAREHOLDERS
			.range(deps.storage, None, None, Ascending)
			.collect::<StdResult<Vec<_>>>()?
			.iter()
			.map(|item| -> Result<WasmMsg, ContractError>  {
				let withdraw_amnt = calculate_withdraw_amnt(&deps, env.clone(), info.clone(), &item.0)?;
				let message = get_withdraw_msg(&deps, env.clone(), info.clone(), withdraw_amnt, &item.0)?;
				SHAREHOLDERS.update(deps.storage, &item.0, |info: Option<ShareholderInfo>| -> Result<ShareholderInfo, ContractError> {
					let mut ret = info.ok_or(ContractError::UnexpectedInput{})?;
					ret.last_withdraw_timestamp = curr_timestamp;
					Ok(ret)
				})?;
				Ok(message)
			})
			.collect::<Result<Vec<_>, ContractError>>()?;
	}
	
	// add new member
	let new_info = ShareholderInfo {
		last_withdraw_timestamp: curr_timestamp,
		weight: weight,
	};
	SHAREHOLDERS.save(deps.storage, addr, &new_info)?;
	
	// emit
	Ok(Response::new()
		.add_messages(msgs)
	)
}

pub fn execute_kickoff(
	deps: DepsMut,
	_env: Env,
	info: MessageInfo,
	date: Timestamp,
) -> Result<Response, ContractError> {
	
	// only admin can kickoff
	let mut config = CONFIG.load(deps.storage)?;
	if config.admin != info.sender {
		return Err(ContractError::Unauthorized{});
	}
	
	if is_kickstarted(config.schedule_start.clone()) {
		return Err(ContractError::ActiveContract{});
	}
	
	config.schedule_start = date;
	CONFIG.save(deps.storage, &config)?;
	
	Ok(Response::new())
	
}

pub fn calculate_withdraw_amnt(
	deps: &DepsMut,
	env: Env,
	info: MessageInfo,
	addr: &Addr,
) -> Result<Uint128, ContractError> {
	
	let config = CONFIG.load(deps.storage)?;
	let member_info = SHAREHOLDERS.load(deps.storage, &addr)?;
	
	// TODO consider moving this into separate function
	// and make save maths on it (save add with overflow error)
	let weight_sum = SHAREHOLDERS
		.range(deps.storage, None, None, Ascending)
		.collect::<StdResult<Vec<_>>>()?
		.iter()
		.map(|item| item.1.weight)
		.sum::<u64>();
		
	let balance = query_balance(&deps, env.clone(), info.clone())?;
	let weight = (
		Uint128::from(member_info.weight),
		Uint128::from(weight_sum)
	);
	let time_diff = Uint128::from(
		env.block.time
		.minus_seconds(member_info.last_withdraw_timestamp.seconds()).seconds()
	);
	let time_weight = (
		time_diff,
		Uint128::from(config.vesting_span),
	);
	Ok(balance
		.checked_mul_floor(time_weight).unwrap_or(Uint128::from(0u64))
		.checked_mul_floor(weight).unwrap_or(Uint128::from(0u64))
	)
	
}

pub fn get_withdraw_msg(
	deps: &DepsMut,
	_env: Env,
	_info: MessageInfo,
	amnt: Uint128,
	dst: &Addr,
) -> StdResult<WasmMsg> {
	
	let config = CONFIG.load(deps.storage)?;
	let send_msg = Cw20ExecuteMsg::Transfer {
		recipient: dst.clone().into_string(),
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
pub fn query(
	deps: Deps,
	env: Env,
	msg: QueryMsg) -> StdResult<Binary> {
    
    match msg {
		QueryMsg::Config {} => {
			Ok(to_binary(&query_config(deps, env)?)?)
		},
	}
    
}

pub fn query_config(
	deps: Deps,
	_env: Env,
) -> StdResult<ContractConfig> {

	Ok(CONFIG.load(deps.storage)?)

}

#[cfg(test)]
mod tests {}
