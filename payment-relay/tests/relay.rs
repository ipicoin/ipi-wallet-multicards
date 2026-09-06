use cosmwasm_schema::cw_serde;
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{
    coins, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdResult,
};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_storage_plus::Item;
use ipi_payment_relay::contract::{execute, instantiate, query};
use ipi_payment_relay::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, VaultConfigResponse, VaultMemberResponse,
    VaultMemberView,
};

const MEMBER: Item<Addr> = Item::new("member");

#[cw_serde]
enum MockVaultQueryMsg {
    Config {},
    Member { address: String },
}

fn mock_vault_instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    MEMBER.save(deps.storage, &info.sender)?;
    Ok(Response::new())
}

fn mock_vault_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}

fn mock_vault_query(deps: Deps, _env: Env, msg: MockVaultQueryMsg) -> StdResult<Binary> {
    match msg {
        MockVaultQueryMsg::Config {} => to_json_binary(&VaultConfigResponse {
            denom: "aipi".to_owned(),
            member_count: 1,
        }),
        MockVaultQueryMsg::Member { address } => {
            let member = MEMBER.load(deps.storage)?;
            let member = (member.as_str() == address).then_some(VaultMemberView { address });
            to_json_binary(&VaultMemberResponse { member })
        }
    }
}

fn relay_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(execute, instantiate, query))
}

fn vault_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        mock_vault_execute,
        mock_vault_instantiate,
        mock_vault_query,
    ))
}

struct Fixture {
    app: App,
    controller: Addr,
    recipient: Addr,
    vault: Addr,
    relay_code_id: u64,
}

fn fixture() -> Fixture {
    let controller = MockApi::default().addr_make("controller");
    let recipient = MockApi::default().addr_make("recipient");
    let mut app = AppBuilder::new().build(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &controller, coins(1_000_000, "aipi"))
            .unwrap();
    });
    let vault_code_id = app.store_code(vault_contract());
    let relay_code_id = app.store_code(relay_contract());
    let vault = app
        .instantiate_contract(
            vault_code_id,
            controller.clone(),
            &Empty {},
            &[],
            "vault",
            None,
        )
        .unwrap();
    Fixture {
        app,
        controller,
        recipient,
        vault,
        relay_code_id,
    }
}

fn instantiate_relay(fixture: &mut Fixture, slot: u8, admin: Option<String>) -> Addr {
    fixture
        .app
        .instantiate_contract(
            fixture.relay_code_id,
            fixture.controller.clone(),
            &InstantiateMsg {
                vault: fixture.vault.to_string(),
                slot,
            },
            &[],
            format!("relay-{slot}"),
            admin,
        )
        .unwrap()
}

#[test]
fn five_relays_forward_the_exact_amount_and_retain_nothing() {
    let mut fixture = fixture();
    let relays = (0..5)
        .map(|slot| instantiate_relay(&mut fixture, slot, None))
        .collect::<Vec<_>>();
    let amount = 42_000u128;
    let response = fixture
        .app
        .execute_contract(
            fixture.controller.clone(),
            relays[0].clone(),
            &ExecuteMsg::Forward {
                controller: fixture.controller.to_string(),
                route: relays[1..].iter().map(ToString::to_string).collect(),
                recipient: fixture.recipient.to_string(),
                payment_id: "a".repeat(64),
            },
            &coins(amount, "aipi"),
        )
        .unwrap();

    assert_eq!(
        fixture
            .app
            .wrap()
            .query_balance(&fixture.recipient, "aipi")
            .unwrap()
            .amount
            .u128(),
        amount
    );
    for relay in &relays {
        assert!(fixture
            .app
            .wrap()
            .query_balance(relay, "aipi")
            .unwrap()
            .amount
            .is_zero());
    }
    let relay_events = response
        .events
        .iter()
        .filter(|event| event.ty == "wasm-ipi_payment_relay")
        .collect::<Vec<_>>();
    assert_eq!(relay_events.len(), 5);
    assert!(relay_events
        .iter()
        .all(|event| event.attributes.iter().any(
            |attribute| attribute.key == "vault" && attribute.value == fixture.vault.as_str()
        )));
    assert!(relay_events
        .last()
        .unwrap()
        .attributes
        .iter()
        .any(|attribute| attribute.key == "action" && attribute.value == "relay_deliver"));

    let config: ConfigResponse = fixture
        .app
        .wrap()
        .query_wasm_smart(&relays[4], &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.vault, fixture.vault.as_str());
    assert_eq!(config.slot, 4);
    assert_eq!(config.relay_count, 5);
}

#[test]
fn only_active_vault_members_can_create_relays() {
    let mut fixture = fixture();
    let error = fixture
        .app
        .instantiate_contract(
            fixture.relay_code_id,
            MockApi::default().addr_make("stranger"),
            &InstantiateMsg {
                vault: fixture.vault.to_string(),
                slot: 0,
            },
            &[],
            "unauthorized-relay",
            None,
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("only an active card"));
}

#[test]
fn route_rejects_duplicate_slots_and_admin_controlled_relays() {
    let mut fixture = fixture();
    let first = instantiate_relay(&mut fixture, 0, None);
    let duplicate_slot = instantiate_relay(&mut fixture, 0, None);
    let relay_two = instantiate_relay(&mut fixture, 2, None);
    let relay_three = instantiate_relay(&mut fixture, 3, None);
    let relay_four = instantiate_relay(&mut fixture, 4, None);
    let duplicate_error = fixture
        .app
        .execute_contract(
            fixture.controller.clone(),
            first.clone(),
            &ExecuteMsg::Forward {
                controller: fixture.controller.to_string(),
                route: vec![
                    duplicate_slot.to_string(),
                    relay_two.to_string(),
                    relay_three.to_string(),
                    relay_four.to_string(),
                ],
                recipient: fixture.recipient.to_string(),
                payment_id: "b".repeat(64),
            },
            &coins(10, "aipi"),
        )
        .unwrap_err();
    assert!(format!("{duplicate_error:?}").contains("duplicate contracts or slots"));

    let controller = fixture.controller.to_string();
    let controlled = instantiate_relay(&mut fixture, 1, Some(controller));
    let controlled_error = fixture
        .app
        .execute_contract(
            fixture.controller.clone(),
            first,
            &ExecuteMsg::Forward {
                controller: fixture.controller.to_string(),
                route: vec![
                    controlled.to_string(),
                    relay_two.to_string(),
                    relay_three.to_string(),
                    relay_four.to_string(),
                ],
                recipient: fixture.recipient.to_string(),
                payment_id: "c".repeat(64),
            },
            &coins(10, "aipi"),
        )
        .unwrap_err();
    assert!(format!("{controlled_error:?}").contains("not a relay for this vault"));
}

#[test]
fn malformed_payment_intent_and_funds_fail_closed() {
    let mut fixture = fixture();
    let relay = instantiate_relay(&mut fixture, 0, None);
    let invalid_id = fixture
        .app
        .execute_contract(
            fixture.controller.clone(),
            relay.clone(),
            &ExecuteMsg::Forward {
                controller: fixture.controller.to_string(),
                route: vec![],
                recipient: fixture.recipient.to_string(),
                payment_id: "not-a-payment-id".to_owned(),
            },
            &coins(10, "aipi"),
        )
        .unwrap_err();
    assert!(format!("{invalid_id:?}").contains("payment_id"));

    let invalid_funds = fixture
        .app
        .execute_contract(
            fixture.controller.clone(),
            relay.clone(),
            &ExecuteMsg::Forward {
                controller: fixture.controller.to_string(),
                route: vec![],
                recipient: fixture.recipient.to_string(),
                payment_id: "d".repeat(64),
            },
            &[],
        )
        .unwrap_err();
    assert!(format!("{invalid_funds:?}").contains("exactly one positive aipi"));

    let incomplete = fixture
        .app
        .execute_contract(
            fixture.controller.clone(),
            relay,
            &ExecuteMsg::Forward {
                controller: fixture.controller.to_string(),
                route: vec![],
                recipient: fixture.recipient.to_string(),
                payment_id: "e".repeat(64),
            },
            &coins(10, "aipi"),
        )
        .unwrap_err();
    assert!(format!("{incomplete:?}").contains("exactly four remaining relays"));
}
