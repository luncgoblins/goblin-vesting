#[cfg(not(feature = "library"))]

use cosmwasm_std::entry_point;
use cosmwasm_std::{
	Order, Empty,
	StdError, Storage,
	Uint64, Timestamp,
	Addr, Binary, Deps, DepsMut, Env, MessageInfo,
	Response, StdResult, QueryRequest, to_binary,
	WasmQuery, Uint128, WasmMsg
};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::query::{QueryMsg, QueryMembersResponse, QueryMemberResponse};
use crate::state::{SHAREHOLDERS, CONFIG, ContractConfig, ShareholderInfo};
use cw20::{BalanceResponse, Cw20QueryMsg, Cw20ExecuteMsg};
use cosmwasm_std::Order::Ascending;

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:goblin-vesting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
	mut _deps: DepsMut,
	_env: Env,
	_msg: Empty,
) -> Result<Response, ContractError> {

	Ok(Response::new())

}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    let config = ContractConfig{
		vesting_period: msg.vesting_period,
		vesting_token_addr: deps.api.addr_validate(
			&msg.token
		)?,
		admin: deps.api.addr_validate(
			&msg.admin
		)?,
		schedule_start: Timestamp::from_seconds(0u64),
		force_withdraw_enabled: msg.force_withdraw_enabled.unwrap_or(false),
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
		ExecuteMsg::ForceWithdraw {} => {
			execute_force_withdraw(deps, env, info)
		},
	}
}

pub fn is_expired(
	start_timestamp: Timestamp,
	current_timestamp: Timestamp,
	duration: Timestamp,
) -> bool {
	
	if is_kickstarted(start_timestamp) {
		return current_timestamp > start_timestamp.plus_seconds(duration.seconds())
	}
	return false;

}

pub fn is_inactive(
	start_timestamp: Timestamp,
	current_timestamp: Timestamp,
) -> bool {

	if !is_kickstarted(start_timestamp) {
		return true;
	}

	current_timestamp < start_timestamp

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
	) {
		return Err(ContractError::InactiveContract{});
	}

	let addr = &info.sender;
	let withdraw_amnt = calculate_withdraw_amnt(deps.as_ref(), env.clone(), addr)?;
	update_last_withdraw_to(deps.storage, env.clone(), addr, env.block.time)?;
	
	let send_request = get_withdraw_msg(
		deps.as_ref(), env.clone(), info.clone(),
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
	if !SHAREHOLDERS.has(deps.storage, addr) {
		return Err(ContractError::UnexpectedInput{});
	}
	
	if is_expired(
		config.schedule_start,
		env.block.time,
		Timestamp::from_seconds(config.vesting_period),
	) {
		return Err(ContractError::ExpiredContract{});
	}
	
	// force withdraw for all members
	// (but only if linear payout started)
	let mut msgs = vec![];
	if !is_inactive(
		config.schedule_start,
		env.block.time,
	) {
		msgs = get_all_withdraw_msgs(deps.as_ref(), env.clone(), info.clone())?;
		update_all_last_withdraw_to(deps.storage, env.clone(), env.block.time)?;
	}

	// remove member
	SHAREHOLDERS.remove(deps.storage, addr);
	
	// compensation message
	let compensation_msg = get_withdraw_msg(
		deps.as_ref(), env.clone(), info.clone(),
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
	if SHAREHOLDERS.has(deps.storage, addr) {
		return Err(ContractError::UnexpectedInput{});
	}
	
	if is_expired(
		config.schedule_start,
		env.block.time,
		Timestamp::from_seconds(config.vesting_period),
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
	) {
		msgs = get_all_withdraw_msgs(deps.as_ref(), env.clone(), info.clone())?;
		update_all_last_withdraw_to(deps.storage, env.clone(), env.block.time)?;
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
	env: Env,
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

	update_all_last_withdraw_to(deps.storage, env, date)?;
	
	config.schedule_start = date;
	CONFIG.save(deps.storage, &config)?;
	
	Ok(Response::new())
	
}

pub fn execute_force_withdraw(
	deps: DepsMut,
	env: Env,
	info: MessageInfo,
) -> Result<Response, ContractError> {

	// only admin can force withdraw
	let config = CONFIG.load(deps.storage)?;
	if config.admin != info.sender {
		return Err(ContractError::Unauthorized{});
	}

	// only from enabled contracts can be
	// force withdrewn
	if !config.force_withdraw_enabled {
		return Err(ContractError::Unauthorized{});
	}

	// force withdraw but only
	// within linear schedule
	let mut msgs = vec![];
	if !is_inactive(
		config.schedule_start,
		env.block.time,
	) {
		msgs = get_all_withdraw_msgs(deps.as_ref(), env.clone(), info.clone())?;
		update_all_last_withdraw_to(deps.storage, env.clone(), env.block.time)?;
	}

	Ok(Response::new()
		.add_messages(msgs)
	)

}

pub fn update_last_withdraw_to(
	store: &mut dyn Storage,
	env: Env,
	addr: &Addr,
	to: Timestamp
) -> StdResult<ShareholderInfo> {

	Ok(SHAREHOLDERS.update(store, addr, |item: Option<ShareholderInfo>| -> StdResult<ShareholderInfo>{
		let mut i = item.ok_or(StdError::GenericErr{msg: String::from("unable")})?;
		i.last_withdraw_timestamp = to;
		Ok(i)
	})?)

}

pub fn update_all_last_withdraw_to(
	store: &mut dyn Storage,
	env: Env,
	to: Timestamp
) -> StdResult<Vec<ShareholderInfo>>{

	Ok(SHAREHOLDERS
		.range(store, None, None, Ascending)
		.collect::<StdResult<Vec<_>>>()?
		.iter()
		.map(|pair| -> StdResult<ShareholderInfo> {
			update_last_withdraw_to(store, env.clone(), &pair.0, to)
		})
		.collect::<StdResult<Vec<_>>>()?
	)

}

pub fn calculate_weight_sum (
	deps: Deps
) -> StdResult<Uint64>{
	
	Ok(
		SHAREHOLDERS
			.range(deps.storage, None, None, Ascending)
			.fold(Ok(Uint64::zero()), |acc: StdResult<Uint64>, item| {
				Ok(acc?.checked_add(Uint64::from(item?.1.weight))?)
			})?
	)
	
}

pub fn calculate_withdraw_amnt(
	deps: Deps,
	env: Env,
	addr: &Addr,
) -> StdResult<Uint128> {
	
	let config = CONFIG.load(deps.storage)?;
	let member_info = SHAREHOLDERS.load(deps.storage, &addr)?;
	let weight_sum = calculate_weight_sum(deps)?;

	if is_inactive(
		config.schedule_start,
		env.block.time
	) {
		return Ok(Uint128::from(0u32));
	}

	let balance = query_balance(deps, env.clone())?;
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
		Uint128::from(config.vesting_period),
	);
	Ok(balance
		.checked_mul_floor(time_weight).unwrap_or(Uint128::from(0u64))
		.checked_mul_floor(weight).unwrap_or(Uint128::from(0u64))
	)
	
}

pub fn get_withdraw_msg(
	deps: Deps,
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

pub fn get_all_withdraw_msgs(
	deps: Deps,
	env: Env,
	info: MessageInfo,
) -> StdResult<Vec<WasmMsg>>{

	let msgs = SHAREHOLDERS
		.range(deps.storage, None, None, Ascending)
		.collect::<StdResult<Vec<_>>>()?
		.iter()
		.map(|item| -> StdResult<WasmMsg>  {
			let withdraw_amnt = calculate_withdraw_amnt(deps, env.clone(), &item.0)?;
			let message = get_withdraw_msg(deps, env.clone(), info.clone(), withdraw_amnt, &item.0)?;
			Ok(message)
		})
		.collect::<StdResult<Vec<_>>>()?;

	Ok(msgs)

}

pub fn query_balance(
	deps: Deps,
	env: Env,
) -> StdResult<Uint128> {
	
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
		QueryMsg::Members {} => {
			Ok(to_binary(&query_members(deps, env)?)?)
		},
		QueryMsg::Member { addr } => {
			Ok(to_binary(&query_member(deps, env, addr)?)?)
		}
	}
    
}

pub fn query_config(
	deps: Deps,
	_env: Env,
) -> StdResult<ContractConfig> {

	Ok(CONFIG.load(deps.storage)?)

}

pub fn query_members(
	deps: Deps,
	_env: Env,
) -> StdResult<QueryMembersResponse> {

	let addresses = SHAREHOLDERS
		.keys(deps.storage, None, None, Order::Ascending)
		.into_iter()
		.map(|item| item.unwrap())
		.collect::<Vec<_>>();
	let resp = QueryMembersResponse {
		members: addresses
	};
	Ok(resp)

}

pub fn query_member(
	deps: Deps,
	env: Env,
	addr: Addr,
) -> StdResult<QueryMemberResponse> {

	let denominator = calculate_weight_sum(deps)?.u64();
	let nominator = SHAREHOLDERS.load(deps.storage, &addr)?.weight;
	let can_withdraw: Uint128 = calculate_withdraw_amnt(deps, env, &addr)?;
	let ret = QueryMemberResponse{
		addr: addr,
		weight_denominator: denominator,
		weight_nominator: nominator,
		can_withdraw: can_withdraw,
	};
	Ok(ret)

}

#[cfg(test)]
mod tests {}
