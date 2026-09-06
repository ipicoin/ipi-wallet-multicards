use cosmwasm_std::{Addr, AnyMsg, Binary, CosmosMsg};
use prost::Message;

const EXECUTE_CONTRACT_TYPE_URL: &str = "/cosmwasm.wasm.v1.MsgExecuteContract";
const BASIC_ALLOWANCE_TYPE_URL: &str = "/cosmos.feegrant.v1beta1.BasicAllowance";
const ALLOWED_MSG_ALLOWANCE_TYPE_URL: &str = "/cosmos.feegrant.v1beta1.AllowedMsgAllowance";
const GRANT_ALLOWANCE_TYPE_URL: &str = "/cosmos.feegrant.v1beta1.MsgGrantAllowance";
const REVOKE_ALLOWANCE_TYPE_URL: &str = "/cosmos.feegrant.v1beta1.MsgRevokeAllowance";

// Enough for one acceptance at the wallet's fixed fee plus one retry. Once a
// card accepts, this bounded invitation grant is replaced atomically by the
// same member allowance used by every active card.
const INVITATION_ALLOWANCE_AIPI: &str = "2400000000000000";

#[derive(Clone, PartialEq, Message)]
struct ProtoAny {
    #[prost(string, tag = "1")]
    type_url: String,
    #[prost(bytes = "vec", tag = "2")]
    value: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
struct ProtoCoin {
    #[prost(string, tag = "1")]
    denom: String,
    #[prost(string, tag = "2")]
    amount: String,
}

#[derive(Clone, PartialEq, Message)]
struct BasicAllowance {
    #[prost(message, repeated, tag = "1")]
    spend_limit: Vec<ProtoCoin>,
}

#[derive(Clone, PartialEq, Message)]
struct AllowedMsgAllowance {
    #[prost(message, optional, tag = "1")]
    allowance: Option<ProtoAny>,
    #[prost(string, repeated, tag = "2")]
    allowed_messages: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
struct MsgGrantAllowance {
    #[prost(string, tag = "1")]
    granter: String,
    #[prost(string, tag = "2")]
    grantee: String,
    #[prost(message, optional, tag = "3")]
    allowance: Option<ProtoAny>,
}

#[derive(Clone, PartialEq, Message)]
struct MsgRevokeAllowance {
    #[prost(string, tag = "1")]
    granter: String,
    #[prost(string, tag = "2")]
    grantee: String,
}

pub fn grant_invitation_fee(granter: &Addr, grantee: &Addr) -> CosmosMsg {
    grant_fee(
        granter,
        grantee,
        vec![ProtoCoin {
            denom: "aipi".to_owned(),
            amount: INVITATION_ALLOWANCE_AIPI.to_owned(),
        }],
    )
}

pub fn grant_member_fees(granter: &Addr, grantee: &Addr) -> CosmosMsg {
    // Active cards already have full 1-of-N authority over the vault balance.
    // The empty BasicAllowance limit therefore mirrors that authority for fees,
    // while AllowedMsgAllowance prevents use for non-CosmWasm transactions.
    grant_fee(granter, grantee, Vec::new())
}

fn grant_fee(granter: &Addr, grantee: &Addr, spend_limit: Vec<ProtoCoin>) -> CosmosMsg {
    let basic = BasicAllowance { spend_limit }.encode_to_vec();
    let allowed = AllowedMsgAllowance {
        allowance: Some(ProtoAny {
            type_url: BASIC_ALLOWANCE_TYPE_URL.to_owned(),
            value: basic,
        }),
        allowed_messages: vec![EXECUTE_CONTRACT_TYPE_URL.to_owned()],
    }
    .encode_to_vec();
    let grant = MsgGrantAllowance {
        granter: granter.to_string(),
        grantee: grantee.to_string(),
        allowance: Some(ProtoAny {
            type_url: ALLOWED_MSG_ALLOWANCE_TYPE_URL.to_owned(),
            value: allowed,
        }),
    };
    CosmosMsg::Any(AnyMsg {
        type_url: GRANT_ALLOWANCE_TYPE_URL.to_owned(),
        value: Binary::new(grant.encode_to_vec()),
    })
}

pub fn revoke_fees(granter: &Addr, grantee: &Addr) -> CosmosMsg {
    let revoke = MsgRevokeAllowance {
        granter: granter.to_string(),
        grantee: grantee.to_string(),
    };
    CosmosMsg::Any(AnyMsg {
        type_url: REVOKE_ALLOWANCE_TYPE_URL.to_owned(),
        value: Binary::new(revoke.encode_to_vec()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unwrap_any(message: CosmosMsg) -> AnyMsg {
        match message {
            CosmosMsg::Any(any) => any,
            _ => panic!("expected protobuf Any message"),
        }
    }

    #[test]
    fn invitation_grant_is_bounded_and_only_allows_contract_execution() {
        let granter = Addr::unchecked("ipi1vault");
        let grantee = Addr::unchecked("ipi1card");
        let any = unwrap_any(grant_invitation_fee(&granter, &grantee));
        assert_eq!(any.type_url, GRANT_ALLOWANCE_TYPE_URL);

        let grant = MsgGrantAllowance::decode(any.value.as_slice()).unwrap();
        assert_eq!(grant.granter, granter.as_str());
        assert_eq!(grant.grantee, grantee.as_str());
        let allowed =
            AllowedMsgAllowance::decode(grant.allowance.unwrap().value.as_slice()).unwrap();
        assert_eq!(allowed.allowed_messages, [EXECUTE_CONTRACT_TYPE_URL]);
        let basic = BasicAllowance::decode(allowed.allowance.unwrap().value.as_slice()).unwrap();
        assert_eq!(basic.spend_limit.len(), 1);
        assert_eq!(basic.spend_limit[0].denom, "aipi");
        assert_eq!(basic.spend_limit[0].amount, INVITATION_ALLOWANCE_AIPI);
    }

    #[test]
    fn member_grant_and_revoke_use_the_vault_as_granter() {
        let granter = Addr::unchecked("ipi1vault");
        let grantee = Addr::unchecked("ipi1card");
        let grant_any = unwrap_any(grant_member_fees(&granter, &grantee));
        let grant = MsgGrantAllowance::decode(grant_any.value.as_slice()).unwrap();
        let allowed =
            AllowedMsgAllowance::decode(grant.allowance.unwrap().value.as_slice()).unwrap();
        let basic = BasicAllowance::decode(allowed.allowance.unwrap().value.as_slice()).unwrap();
        assert!(basic.spend_limit.is_empty());

        let revoke_any = unwrap_any(revoke_fees(&granter, &grantee));
        assert_eq!(revoke_any.type_url, REVOKE_ALLOWANCE_TYPE_URL);
        let revoke = MsgRevokeAllowance::decode(revoke_any.value.as_slice()).unwrap();
        assert_eq!(revoke.granter, granter.as_str());
        assert_eq!(revoke.grantee, grantee.as_str());
    }
}
