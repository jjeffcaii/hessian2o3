#[macro_use]
extern crate log;

mod encode;

pub type Result<T> = anyhow::Result<T>;

pub use encode::*;

#[cfg(test)]
mod tests {
    use super::*;

    use bytes::BytesMut;

    #[test]
    fn test_encode() {
        let mut buf = BytesMut::with_capacity(1024);

        1234i32.encode(&mut buf);
    }
}
