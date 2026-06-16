use std::fmt::Display;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] io::Error),

    #[error("unknown hessian2 serde error")]
    Unknown,
}

impl Error {
    pub(crate) fn io(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl serde::ser::Error for Error {
    fn custom<T: Display>(_msg: T) -> Self {
        Self::Unknown
    }
}
