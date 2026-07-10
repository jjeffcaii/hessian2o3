use std::fmt::Display;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] io::Error),

    #[error("unknown hessian2 serde error")]
    Unknown,

    #[error("{0}")]
    Custom(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Custom(format!("{}", msg))
    }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Custom(format!("{}", msg))
    }
}
