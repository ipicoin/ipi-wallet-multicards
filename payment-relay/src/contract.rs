#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, BankMsg, Coin, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
    WasmMsg,
};
use cw2::set_contract_version;
use std::collections::BTreeSet;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, VaultConfigResponse, VaultMemberResponse,
    VaultQueryMsg,
};
use crate::state::{Config, CONFIG};

const CONTRACT_NAME: &str = "crates.io:ipi-payment-relay";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const IPI_DENOM: &str = "aipi";
const RELAY_COUNT: u8 = 5;
const MAX_REMAINING_RELAYS: usize = RELAY_COUNT as usize - 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if !info.funds.is_empty() {
        return Err(ContractError::InvalidFunds);
    }
    if msg.slot >= RELAY_COUNT {
        return Err(ContractError::InvalidSlot);
    }
    let vault = deps.api.addr_validate(&msg.vault)?;
    let vault_config: VaultConfigResponse = deps
        .querier
        .query_wasm_smart(&vault, &VaultQueryMsg::Config {})?;
    if vault_config.denom != IPI_DENOM {
        return Err(ContractError::WrongVaultDenom);
    }
    ensure_vault_member(deps.as_ref(), &vault, &info.sender)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(
        deps.storage,
        &Config {
            vault: vault.clone(),
            slot: msg.slot,
        },
    )?;
    Ok(Response::new()
        .add_attribute("action", "instantiate_payment_relay")
        .add_attribute("vault", vault)
        .add_attribute("controller", info.sender)
        .add_attribute("slot", msg.slot.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Forward {
            controller,
            route,
            recipient,
            payment_id,
        } => execute_forward(deps, env, info, controller, route, recipient, payment_id),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_forward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    controller: String,
    route: Vec<String>,
    recipient: String,
    payment_id: String,
) -> Result<Response, ContractError> {
    let coin = one_ipi_coin(&info)?;
    validate_payment_id(&payment_id)?;
    if route.len() > MAX_REMAINING_RELAYS {
        return Err(ContractError::RouteTooLong);
    }

    let config = CONFIG.load(deps.storage)?;
    let controller = deps.api.addr_validate(&controller)?;
    let recipient = deps.api.addr_validate(&recipient)?;
    ensure_vault_member(deps.as_ref(), &config.vault, &controller)?;

    if info.sender == controller {
        if route.len() != MAX_REMAINING_RELAYS {
            return Err(ContractError::IncompleteRoute);
        }
    } else {
        ensure_compatible_relay(deps.as_ref(), &env, &config.vault, &info.sender)?;
    }

    let mut seen_addresses = BTreeSet::from([env.contract.address.to_string()]);
    let mut seen_slots = BTreeSet::from([config.slot]);
    let mut validated_route = Vec::with_capacity(route.len());
    for candidate in route {
        let address = deps.api.addr_validate(&candidate)?;
        if !seen_addresses.insert(address.to_string()) {
            return Err(ContractError::DuplicateRelay);
        }
        let relay = compatible_relay_config(deps.as_ref(), &env, &config.vault, &address)?;
        if !seen_slots.insert(relay.slot) {
            return Err(ContractError::DuplicateRelay);
        }
        validated_route.push(address);
    }

    let next = validated_route.first().cloned();
    let remaining = validated_route
        .iter()
        .skip(1)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let destination = next
        .as_ref()
        .map_or_else(|| recipient.to_string(), ToString::to_string);
    let action = if next.is_some() {
        "relay_forward"
    } else {
        "relay_deliver"
    };
    let event = payment_event(
        action,
        &payment_id,
        &config,
        &env.contract.address,
        &controller,
        &recipient,
        &destination,
        &coin,
        validated_route.len(),
    );

    let response = Response::new().add_event(event);
    if let Some(next) = next {
        Ok(response.add_message(WasmMsg::Execute {
            contract_addr: next.to_string(),
            msg: to_json_binary(&ExecuteMsg::Forward {
                controller: controller.to_string(),
                route: remaining,
                recipient: recipient.to_string(),
                payment_id,
            })?,
            funds: vec![coin],
        }))
    } else {
        Ok(response.add_message(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: vec![coin],
        }))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<cosmwasm_std::Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        vault: config.vault.to_string(),
        slot: config.slot,
        denom: IPI_DENOM.to_owned(),
        relay_count: RELAY_COUNT,
    })
}

fn one_ipi_coin(info: &MessageInfo) -> Result<Coin, ContractError> {
    match info.funds.as_slice() {
        [coin] if coin.denom == IPI_DENOM && !coin.amount.is_zero() => Ok(coin.clone()),
        _ => Err(ContractError::InvalidFunds),
    }
}

fn validate_payment_id(payment_id: &str) -> Result<(), ContractError> {
    if payment_id.len() != 64
        || !payment_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContractError::InvalidPaymentId);
    }
    Ok(())
}

fn ensure_vault_member(
    deps: Deps,
    vault: &cosmwasm_std::Addr,
    controller: &cosmwasm_std::Addr,
) -> Result<(), ContractError> {
    let response: VaultMemberResponse = deps.querier.query_wasm_smart(
        vault,
        &VaultQueryMsg::Member {
            address: controller.to_string(),
        },
    )?;
    if response.member.is_none() {
        return Err(ContractError::UnauthorizedController);
    }
    Ok(())
}

fn compatible_relay_config(
    deps: Deps,
    env: &Env,
    vault: &cosmwasm_std::Addr,
    relay: &cosmwasm_std::Addr,
) -> Result<ConfigResponse, ContractError> {
    let own_info = deps
        .querier
        .query_wasm_contract_info(&env.contract.address)?;
    let relay_info = deps.querier.query_wasm_contract_info(relay)?;
    if own_info.code_id != relay_info.code_id || relay_info.admin.is_some() {
        return Err(ContractError::UntrustedRelay);
    }
    let relay_config: ConfigResponse =
        deps.querier.query_wasm_smart(relay, &QueryMsg::Config {})?;
    if relay_config.vault != vault.as_str()
        || relay_config.denom != IPI_DENOM
        || relay_config.relay_count != RELAY_COUNT
        || relay_config.slot >= RELAY_COUNT
    {
        return Err(ContractError::UntrustedRelay);
    }
    Ok(relay_config)
}

fn ensure_compatible_relay(
    deps: Deps,
    env: &Env,
    vault: &cosmwasm_std::Addr,
    relay: &cosmwasm_std::Addr,
) -> Result<(), ContractError> {
    compatible_relay_config(deps, env, vault, relay).map(|_| ())
}

#[allow(clippy::too_many_arguments)]
fn payment_event(
    action: &str,
    payment_id: &str,
    config: &Config,
    relay: &cosmwasm_std::Addr,
    controller: &cosmwasm_std::Addr,
    recipient: &cosmwasm_std::Addr,
    next: &str,
    coin: &Coin,
    remaining_hops: usize,
) -> Event {
    Event::new("ipi_payment_relay")
        .add_attribute("action", action)
        .add_attribute("payment_id", payment_id)
        .add_attribute("vault", config.vault.to_string())
        .add_attribute("controller", controller.to_string())
        .add_attribute("relay", relay.to_string())
        .add_attribute("slot", config.slot.to_string())
        .add_attribute("next", next)
        .add_attribute("recipient", recipient.to_string())
        .add_attribute("amount", coin.amount.to_string())
        .add_attribute("denom", coin.denom.clone())
        .add_attribute("remaining_hops", remaining_hops.to_string())
}
