use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub vault: String,
    pub slot: u8,
}

#[cw_serde]
pub enum ExecuteMsg {
    Forward {
        controller: String,
        route: Vec<String>,
        recipient: String,
        payment_id: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub vault: String,
    pub slot: u8,
    pub denom: String,
    pub relay_count: u8,
}

#[cw_serde]
pub enum VaultQueryMsg {
    Config {},
    Member { address: String },
}

#[cw_serde]
pub struct VaultConfigResponse {
    pub denom: String,
    pub member_count: u64,
}

#[cw_serde]
pub struct VaultMemberView {
    pub address: String,
}

#[cw_serde]
pub struct VaultMemberResponse {
    pub member: Option<VaultMemberView>,
}
