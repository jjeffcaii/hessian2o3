# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This
project is still a work in progress and does not yet promise strict
[Semantic Versioning](https://semver.org/) guarantees between `0.0.x` releases.

## [0.0.8] - 2026-07-20

### Added

- `serialize!` macro — auto-selects the encoding from the value's type: uses its
  `HSerialize` impl when it has one, otherwise the `serde::Serialize` path
- `deserialize!` macro — auto-selects the decoding from the inferred target type `T`:
  uses `T`'s `HDeserialize` impl when it has one, otherwise `serde` (backed by the new
  `AutoDeserialize` trait)
- `#[derive(HessianDeserialize)]` — new derive generating the `HDeserialize` (decode) impl,
  split out from `#[derive(HessianSerialize)]`
- `get_value` / `get_value_from_slice` — dedicated readers that decode Hessian bytes into a
  `Value` with full fidelity, preserving the native `Date` primitive
- `#[hessian(date)]` field attribute for the `HessianSerialize` derive — encodes an `i64`
  (Unix milliseconds) field as the native Hessian date wire type (`0x4a`/`0x4b`), the derive
  parallel to `#[serde(with = "hessian2::date")]`; decoding is unchanged since an `i64` field
  already accepts a date value
- `Encoder::put_date_millis` — public method to encode a date from raw Unix milliseconds

### Changed

- **Breaking:** `#[derive(HessianSerialize)]` now generates only the `HSerialize` (encode)
  impl; derive `HessianDeserialize` as well to get the decode side (previously the single
  derive generated both)
- `from_reader`/`from_slice` no longer special-case `Value`; every type decodes uniformly
  through `serde`. Use `get_value`/`get_value_from_slice` for date-faithful `Value` decoding
  (the `serde` path decodes a date tag as a plain long). The now-unnecessary `T: 'static`
  bound was dropped from both functions

## [0.0.7] - 2026-07-17

### Added

- Hessian `Date` support via `#[serde(with = "hessian2::date")]` on an `i64` (Unix
  milliseconds) field, encoding it as the native Hessian date wire tag (`0x4a`/`0x4b`)
  instead of a plain long; decoding accepts either wire flavor

### Fixed

- `PrimitiveValue::Date` now round-trips correctly through `to_vec`/`from_slice` and
  `to_value`/`from_value` instead of silently degrading to `PrimitiveValue::Long`

## [0.0.6] - 2026-07-17

### Changed

- Renamed the `HessianSerialize`/`HessianDeserialize` traits to `HSerialize`/`HDeserialize`,
  and renamed the `#[derive(Hessian)]` macro to `#[derive(HessianSerialize)]`
- `to_vec`/`to_writer` now dispatch through a `HessianWriteable` trait; wrap a value as
  `Hessian(&value)` to make them prefer its `HSerialize` impl over `serde::Serialize` when a
  type implements both
- `PrimitiveValue::Date` now stores raw Unix milliseconds instead of `SystemTime`
- Upgraded `hessian2-derive` to `syn` 2.0

### Added

- `hessian2::prelude` module for importing the common traits and derive macro in one line

### Removed

- `HessianDate` wrapper type, superseded by `PrimitiveValue::Date` storing millis directly

### Fixed

- `cargo clippy` lints around the visibility of internal `Formatter`/`Serializer` types,
  redundant references in `format!`/`info!` arguments, and a byte-string literal suggestion

## [0.0.5] - 2026-07-17

### Added

- Hessian back-reference (`'R'`) decoding support
- Value-preserving `from_reader`/`from_value` conversions

## [0.0.4] - 2026-07-14

### Added

- Decoding support for fixed- and variable-length list tags (`0x55`-`0x58`)

## [0.0.3] - 2026-07-10

### Changed

- Refactored to the new `Decoder`/`Encoder` codec APIs

### Fixed

- Resolved all `cargo clippy` warnings

## [0.0.2] - 2026-07-09

### Changed

- Codec refinements
- Polished the README and translated remaining Chinese comments to English

## [0.0.1] - 2026-07-08

Initial release.

### Added

- Core Hessian 2.0 binary encoding/decoding (compact int/long/double forms, chunked
  strings/binary, typed and untyped lists/maps, object codec)
- `#[derive(Hessian)]` macro mapping Rust structs to Java classes
- serde `Serialize`/`Deserialize` integration (`to_vec`, `to_writer`, `from_slice`, `from_reader`)
- Dynamic `Value` type with indexing and `Display` support
- `hessian!` macro for building `Value` literals
- Runnable examples and CI workflow
