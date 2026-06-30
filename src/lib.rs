#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::from_over_into)]
#![allow(clippy::module_inception)]

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
#[macro_use]
extern crate log;

/// cached string
pub(crate) mod cachestr {
    include!(concat!(env!("OUT_DIR"), "/cachestr.rs"));
}

pub mod codec;
pub(crate) mod error;
pub mod hessian;
pub(crate) mod ser;
pub(crate) mod serde;
pub mod value;

pub use error::Error;
pub use hessian::{HessianSerialize, hessian_to_vec, hessian_to_writer};
pub use hessian2o3_derive::HessianObject;

pub type Result<T> = std::result::Result<T, error::Error>;

pub use serde::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_to_vec() -> Result<()> {
        init();

        let v = to_vec(&vec!["foo", "bar", "qux"]);
        assert!(v.is_ok());

        info!("result: {:?}", v.map(|it| hex::encode(&it)));
        Ok(())
    }

    #[test]
    fn test_to_writer() -> Result<()> {
        init();

        let mut buf = vec![];
        let v = to_writer(&mut buf, &vec!["foo", "bar", "qux"]);
        assert!(v.is_ok());
        assert!(!buf.is_empty());

        info!("result: {}", hex::encode(&buf));

        Ok(())
    }
}
