extern crate thiserror;
use thiserror::Error;

/// Specifies library's internal error codes.
///
///
#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Chaining group itself is forbidden.")]
    InvalidChaining,
    #[error("Invalidated group handle.")]
    InvalidGroupHandle,
    #[error("Item name is invalid.")]
    InvalidItemName,
    #[error("Validated group which can execute task is not exist.")]
    NoValidatedGroups,
}
