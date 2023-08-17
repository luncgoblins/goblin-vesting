use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    
    #[error("InitializeFailed")]
    InitializeError {},
    
    #[error("UnexpectedInput")]
    UnexpectedInput {},
    
    #[error("ExpiredContract")]
    ExpiredContract{},
    
    #[error("InactiveContract")]
    InactiveContract{},
    
    #[error("ActiveContract")]
    ActiveContract{},

}
