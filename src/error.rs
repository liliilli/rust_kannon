extern crate thiserror;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Can not chain group itself.")]
    InvalidChaining,
    #[error("Invalidated group handle.")]
    InvalidGroupHandle,
}
