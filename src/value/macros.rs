use super::{Map, Object, PrimitiveValue, Value};
use crate::cachestr::Cachestr;
use crate::codec::Fields;
use serde::Serialize;

/// Converts any `Serialize` value into a [`Value`] using this crate's
/// [`Serializer`](super::ser::Serializer).
///
/// Used by the [`hessian!`](crate::hessian) macro to embed arbitrary
/// expressions (variables, struct literals, numbers, ...) as leaves.
pub fn to_value<T>(value: &T) -> crate::Result<Value>
where
    T: Serialize,
{
    let mut ser = super::ser::Serializer::default();
    value.serialize(&mut ser)
}

/// Key used to mark a `hessian!` object literal as a [`Value::Object`]
/// rather than a plain [`Value::Map`]. Its value becomes the object's class
/// name; every other entry becomes a field, in declaration order.
const CLASS_KEY: &str = "$class";

/// Accumulates the entries of a `hessian!` object literal (in declaration
/// order) and, once complete, decides whether to build a [`Value::Map`] or,
/// if a `"$class"` entry was present, a [`Value::Object`].
///
/// This is an internal helper for the [`hessian!`](crate::hessian) macro
/// and not meant to be used directly.
#[doc(hidden)]
#[derive(Default)]
pub struct ObjectBuilder {
    entries: Vec<(PrimitiveValue, Value)>,
}

impl ObjectBuilder {
    #[doc(hidden)]
    pub fn new() -> Self {
        Self::default()
    }

    #[doc(hidden)]
    pub fn insert(&mut self, key: PrimitiveValue, value: Value) {
        self.entries.push((key, value));
    }

    #[doc(hidden)]
    pub fn into_value(self) -> Value {
        let class_pos = self
            .entries
            .iter()
            .position(|(k, _)| matches!(k, PrimitiveValue::String(s) if s.as_str() == CLASS_KEY));

        let Some(pos) = class_pos else {
            let mut map = Map::with_capacity(self.entries.len());
            for (k, v) in self.entries {
                map.insert(k, v);
            }
            return Value::Map(map);
        };

        let mut entries = self.entries;
        let (_, class_value) = entries.remove(pos);
        let class = match class_value {
            Value::Primitive(PrimitiveValue::String(s)) => s,
            _ => panic!("`$class` value in hessian! must be a string"),
        };

        let mut fields = Fields::new();
        let mut values = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            let name = match k {
                PrimitiveValue::String(s) => s,
                _ => panic!("object field name in hessian! must be a string"),
            };
            fields.push(Cachestr::from(name.as_str()));
            values.push(v);
        }

        Value::Object(Object::new(Cachestr::from(class.as_str()), fields, values))
    }
}

/// Construct a [`Value::Map`], [`Value::List`], or scalar [`Value`] via a
/// JSON-like literal syntax, similar to `serde_json`'s `json!` macro.
///
/// A `"$class"` entry in an object literal is special-cased: instead of a
/// [`Value::Map`], the object is built as a [`Value::Object`] with that
/// entry's value as the class name and every other entry as a field, in
/// declaration order.
///
/// ```
/// use hessian2o3::hessian;
///
/// let v = hessian!({
///     "id": 123,
///     "name": "Jerry",
///     "age": 18,
/// });
///
/// let obj = hessian!({
///     "$class": "com.example.User",
///     "id": 123,
///     "name": "Jerry",
///     "age": 18,
/// });
/// ```
#[macro_export]
macro_rules! hessian {
    ($($tt:tt)+) => {
        $crate::hessian_internal!($($tt)+)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! hessian_internal {
    //////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an array [...]. Produces a
    // vec![...] of the elements.
    //
    // Must be invoked as: hessian_internal!(@array [] $($tt)*)
    //////////////////////////////////////////////////////////////////////

    // Done with trailing comma.
    (@array [$($elems:expr,)*]) => {
        $crate::hessian_internal_vec![$($elems,)*]
    };

    // Done without trailing comma.
    (@array [$($elems:expr),*]) => {
        $crate::hessian_internal_vec![$($elems),*]
    };

    // Next element is `null`.
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::hessian_internal!(@array [$($elems,)* $crate::hessian_internal!(null)] $($rest)*)
    };

    // Next element is `true`.
    (@array [$($elems:expr,)*] true $($rest:tt)*) => {
        $crate::hessian_internal!(@array [$($elems,)* $crate::hessian_internal!(true)] $($rest)*)
    };

    // Next element is `false`.
    (@array [$($elems:expr,)*] false $($rest:tt)*) => {
        $crate::hessian_internal!(@array [$($elems,)* $crate::hessian_internal!(false)] $($rest)*)
    };

    // Next element is an array.
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::hessian_internal!(@array [$($elems,)* $crate::hessian_internal!([$($array)*])] $($rest)*)
    };

    // Next element is a map.
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::hessian_internal!(@array [$($elems,)* $crate::hessian_internal!({$($map)*})] $($rest)*)
    };

    // Next element is an expression followed by comma.
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::hessian_internal!(@array [$($elems,)* $crate::hessian_internal!($next),] $($rest)*)
    };

    // Last element is an expression with no trailing comma.
    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::hessian_internal!(@array [$($elems,)* $crate::hessian_internal!($last)])
    };

    // Comma after the most recent element.
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::hessian_internal!(@array [$($elems,)*] $($rest)*)
    };

    // Unexpected token after most recent element.
    (@array [$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
        $crate::hessian_unexpected!($unexpected)
    };

    //////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an object {...}. Each entry is
    // inserted into the given map variable.
    //
    // Must be invoked as: hessian_internal!(@object $map () ($($tt)*) ($($tt)*))
    //
    // We require two copies of the input tokens so that we can match on one
    // copy and trigger errors on the other copy.
    //////////////////////////////////////////////////////////////////////

    // Done.
    (@object $object:ident () () ()) => {};

    // Insert the current entry followed by trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(($($key)+).into(), $value);
        $crate::hessian_internal!(@object $object () ($($rest)*) ($($rest)*));
    };

    // Current entry followed by unexpected token.
    (@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        $crate::hessian_unexpected!($unexpected);
    };

    // Insert the last entry without trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.insert(($($key)+).into(), $value);
    };

    // Next value is `null`.
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::hessian_internal!(@object $object [$($key)+] ($crate::hessian_internal!(null)) $($rest)*);
    };

    // Next value is `true`.
    (@object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::hessian_internal!(@object $object [$($key)+] ($crate::hessian_internal!(true)) $($rest)*);
    };

    // Next value is `false`.
    (@object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::hessian_internal!(@object $object [$($key)+] ($crate::hessian_internal!(false)) $($rest)*);
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::hessian_internal!(@object $object [$($key)+] ($crate::hessian_internal!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::hessian_internal!(@object $object [$($key)+] ($crate::hessian_internal!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::hessian_internal!(@object $object [$($key)+] ($crate::hessian_internal!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::hessian_internal!(@object $object [$($key)+] ($crate::hessian_internal!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::hessian_internal!();
    };

    // Missing colon and value for last entry. Trigger a reasonable error
    // message.
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::hessian_internal!();
    };

    // Misplaced colon. Trigger a reasonable error message.
    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        $crate::hessian_unexpected!($colon);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        $crate::hessian_unexpected!($comma);
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::hessian_internal!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Refuse to absorb colon token into key expression.
    (@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        $crate::hessian_expect_expr_comma!($($unexpected)+);
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::hessian_internal!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    //////////////////////////////////////////////////////////////////////
    // The main implementation.
    //
    // Must be invoked as: hessian_internal!($($hessian)+)
    //////////////////////////////////////////////////////////////////////

    (null) => {
        $crate::value::Value::Null
    };

    (true) => {
        $crate::value::Value::from(true)
    };

    (false) => {
        $crate::value::Value::from(false)
    };

    ([]) => {
        $crate::value::Value::List($crate::value::List::from($crate::hessian_internal_vec![]))
    };

    ([ $($tt:tt)+ ]) => {
        $crate::value::Value::List($crate::value::List::from($crate::hessian_internal!(@array [] $($tt)+)))
    };

    ({}) => {
        $crate::value::Value::Map($crate::value::Map::new())
    };

    ({ $($tt:tt)+ }) => {
        {
            let mut object = $crate::value::ObjectBuilder::new();
            $crate::hessian_internal!(@object object () ($($tt)+) ($($tt)+));
            object.into_value()
        }
    };

    // Any Serialize type: numbers, strings, struct literals, variables etc.
    // Must be below every other rule.
    ($other:expr) => {
        $crate::value::to_value(&$other).unwrap()
    };
}

// The hessian_internal macro above cannot invoke vec directly because it is
// itself invoked via $crate:: from other macros. Route through a dedicated
// exported macro so `vec!` always resolves to the standard library one.
#[macro_export]
#[doc(hidden)]
macro_rules! hessian_internal_vec {
    ($($content:tt)*) => {
        vec![$($content)*]
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! hessian_unexpected {
    () => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! hessian_expect_expr_comma {
    ($e:expr , $($tt:tt)*) => {};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cachestr::Cachestr;
    use crate::value::{List, Map, Object, PrimitiveValue};
    use smallvec::smallvec;

    fn init() {
        pretty_env_logger::try_init_timed().ok();
    }

    #[test]
    fn test_hessian_macro_object() {
        init();

        let v = hessian!({
            "id": 123,
            "name": "Jerry",
            "age": 18,
        });

        let mut expect = Map::default();
        expect.insert(PrimitiveValue::String("id".to_owned()), Value::from(123i32));
        expect.insert(
            PrimitiveValue::String("name".to_owned()),
            Value::from("Jerry".to_owned()),
        );
        expect.insert(PrimitiveValue::String("age".to_owned()), Value::from(18i32));

        assert_eq!(Value::Map(expect), v);
    }

    #[test]
    fn test_hessian_macro_scalars_and_containers() {
        init();

        assert_eq!(Value::Null, hessian!(null));
        assert_eq!(Value::from(true), hessian!(true));
        assert_eq!(Value::from(false), hessian!(false));
        assert_eq!(Value::from(123i32), hessian!(123));
        assert_eq!(Value::from("foo".to_owned()), hessian!("foo"));

        let v = hessian!([1, "two", [3, 4], null]);
        let expect = Value::List(List::from(vec![
            Value::from(1i32),
            Value::from("two".to_owned()),
            Value::List(List::from(vec![Value::from(3i32), Value::from(4i32)])),
            Value::Null,
        ]));
        assert_eq!(expect, v);

        let v = hessian!({});
        assert_eq!(Value::Map(Map::default()), v);

        let v = hessian!([]);
        assert_eq!(Value::List(List::default()), v);
    }

    #[test]
    fn test_hessian_macro_class_object() {
        init();

        let v = hessian!({
            "$class": "com.example.User",
            "id": 123,
            "name": "Jerry",
            "age": 18,
        });

        let expect = Object::new(
            Cachestr::from("com.example.User"),
            smallvec![
                Cachestr::from("id"),
                Cachestr::from("name"),
                Cachestr::from("age"),
            ],
            vec![
                Value::from(123i32),
                Value::from("Jerry".to_owned()),
                Value::from(18i32),
            ],
        );

        assert_eq!(Value::Object(expect), v);
    }

    #[test]
    fn test_hessian_macro_nested_class_object() {
        init();

        let v = hessian!({
            "name": "Alice",
            "home": {
                "$class": "com.example.Address",
                "city": "Shanghai",
            },
        });

        let home = Object::new(
            Cachestr::from("com.example.Address"),
            smallvec![Cachestr::from("city")],
            vec![Value::from("Shanghai".to_owned())],
        );

        let mut expect = Map::default();
        expect.insert(
            PrimitiveValue::String("name".to_owned()),
            Value::from("Alice".to_owned()),
        );
        expect.insert(
            PrimitiveValue::String("home".to_owned()),
            Value::Object(home),
        );

        assert_eq!(Value::Map(expect), v);
    }

    #[test]
    fn test_hessian_macro_nested_object_and_variable() {
        init();

        let age = 18i32;

        let v = hessian!({
            "user": {
                "name": "Jerry",
                "age": age,
            },
            "roles": ["admin", "user"],
        });

        let mut user = Map::default();
        user.insert(
            PrimitiveValue::String("name".to_owned()),
            Value::from("Jerry".to_owned()),
        );
        user.insert(PrimitiveValue::String("age".to_owned()), Value::from(18i32));

        let mut expect = Map::default();
        expect.insert(PrimitiveValue::String("user".to_owned()), Value::Map(user));
        expect.insert(
            PrimitiveValue::String("roles".to_owned()),
            Value::List(List::from(vec![
                Value::from("admin".to_owned()),
                Value::from("user".to_owned()),
            ])),
        );

        assert_eq!(Value::Map(expect), v);
    }
}
