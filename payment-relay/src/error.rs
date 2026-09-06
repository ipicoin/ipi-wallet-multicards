use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("the configured vault does not use aipi")]
    WrongVaultDenom,

    #[error("only an active card can create or start a relay route")]
    UnauthorizedController,

    #[error("relay slot must be between 0 and 4")]
    InvalidSlot,

    #[error("exactly one positive aipi coin is required")]
    InvalidFunds,

    #[error("payment_id must contain exactly 64 lowercase hexadecimal characters")]
    InvalidPaymentId,

    #[error("a route may contain at most four remaining relays")]
    RouteTooLong,

    #[error("a controller must start a route with exactly four remaining relays")]
    IncompleteRoute,

    #[error("a relay route cannot contain duplicate contracts or slots")]
    DuplicateRelay,

    #[error("the route contains a contract that is not a relay for this vault")]
    UntrustedRelay,
}
