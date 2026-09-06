use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const MEMBER_COUNT: Item<u64> = Item::new("member_count");
pub const MEMBERS: Map<&Addr, ()> = Map::new("members");
pub const INVITATIONS: Map<&Addr, ()> = Map::new("invitations");
