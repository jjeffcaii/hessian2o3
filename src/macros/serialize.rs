use crate::Result;
use crate::hessian::HSerialize;
use crate::serde::{Hessian, to_writer};
use serde::Serialize;
use std::io;

// --- `serialize!` dispatch ------------------------------------------------
//
// A macro cannot inspect what traits its argument implements, and there is no
// stable trait specialization, so we lean on autoref-based specialization
// (dtolnay's trick): two traits expose the same method at different autoref
// levels. `SerializeViaHessian` is implemented for `T` (receiver `&T`, one
// autoref) while `SerializeViaSerde` is implemented for `&T` (receiver `&&T`,
// two autorefs). When both apply, method resolution prefers the receiver
// needing fewer autorefs, so a value implementing `HSerialize` takes the
// Hessian path; anything else falls back to the `serde` path. The macro drives
// this by calling `(&value).__serialize(writer)`.

#[doc(hidden)]
pub trait SerializeViaHessian {
    fn __serialize<W: io::Write>(&self, writer: W) -> Result<()>;
}

impl<T> SerializeViaHessian for T
where
    T: ?Sized + HSerialize,
{
    #[inline]
    fn __serialize<W: io::Write>(&self, writer: W) -> Result<()> {
        to_writer(writer, &Hessian(self))
    }
}

#[doc(hidden)]
pub trait SerializeViaSerde {
    fn __serialize<W>(&self, writer: W) -> Result<()>
    where
        W: io::Write;
}

impl<T> SerializeViaSerde for &T
where
    T: ?Sized + Serialize,
{
    #[inline]
    fn __serialize<W>(&self, writer: W) -> Result<()>
    where
        W: io::Write,
    {
        to_writer(writer, *self)
    }
}

/// Serialize `value` into `writer`, automatically choosing the encoding:
/// if the value's type implements [`HSerialize`], it is wrapped in
/// [`Hessian`] and encoded via that; otherwise it goes through `serde`.
///
/// ```ignore
/// serialize!(&mut writer, value)?;
/// ```
#[macro_export]
macro_rules! serialize {
    ($writer:expr, $value:expr $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::{SerializeViaHessian as _, SerializeViaSerde as _};
        match &$value {
            value => value.__serialize($writer),
        }
    }};
}
