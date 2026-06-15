#[macro_use]
extern crate log;

mod encode;

pub type Result<T> = anyhow::Result<T>;

pub use encode::*;
