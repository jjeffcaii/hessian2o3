pub(crate) const BC_NULL: u8 = b'N';

pub(crate) const BC_MAP: u8 = b'M';
pub(crate) const BC_MAP_UNTYPED: u8 = b'H';

pub(crate) const BC_END: u8 = b'Z';

pub(crate) const BC_LIST_DIRECT: u8 = 0x70;
pub(crate) const BC_LIST_DIRECT_UNTYPED: u8 = 0x78;

pub(crate) const BC_LIST_VARIABLE: u8 = 0x55;
pub(crate) const BC_LIST_FIXED: u8 = b'V';

pub(crate) const BC_LIST_VARIABLE_UNTYPED: u8 = 0x57;
pub(crate) const BC_LIST_FIXED_UNTYPED: u8 = 0x58;

pub(crate) const LIST_DIRECT_MAX: usize = 7;

pub(crate) const BC_BOOL_TRUE: u8 = b'T';
pub(crate) const BC_BOOL_FALSE: u8 = b'F';

pub(crate) const BC_DATE_MINUTE: u8 = 0x4b;
pub(crate) const BC_DATE: u8 = 0x4a;

pub(crate) const BC_LONG: u8 = b'L';
pub(crate) const BC_LONG_ZERO: u8 = 0xe0;
pub(crate) const BC_LONG_BYTE_ZERO: u8 = 0xf8;
pub(crate) const BC_LONG_SHORT_ZERO: u8 = 0x3c;
pub(crate) const BC_LONG_INT: u8 = 0x59;

pub(crate) const BC_INT: u8 = b'I';
pub(crate) const BC_INT_ZERO: u8 = 0x90;
pub(crate) const BC_INT_BYTE_ZERO: u8 = 0xc8;
pub(crate) const BC_INT_SHORT_ZERO: u8 = 0xd4;

pub(crate) const BC_DOUBLE: u8 = b'D';
pub(crate) const BC_DOUBLE_ZERO: u8 = 0x5b;
pub(crate) const BC_DOUBLE_ONE: u8 = 0x5c;
pub(crate) const BC_DOUBLE_BYTE: u8 = 0x5d;
pub(crate) const BC_DOUBLE_SHORT: u8 = 0x5e;
pub(crate) const BC_DOUBLE_MILL: u8 = 0x5f;

pub(crate) const BC_STRING_DIRECT: u8 = 0x00;
pub(crate) const BC_STRING_SHORT: u8 = 0x30;
pub(crate) const BC_STRING_CHUNK: u8 = b'R'; // non-final string
pub(crate) const BC_STRING: u8 = b'S'; // final string

pub(crate) const BC_BINARY: u8 = b'B'; // final chunk
pub(crate) const BC_BINARY_CHUNK: u8 = b'A'; // non-final chunk
pub(crate) const BC_BINARY_DIRECT: u8 = 0x20; // 1-byte length binary
pub(crate) const BC_BINARY_SHORT: u8 = 0x34; // 2-byte length binary

pub(crate) const BINARY_DIRECT_MAX: usize = 0x0f;
pub(crate) const BINARY_SHORT_MAX: usize = 0x3ff; // 0-1023 binary

pub(crate) const BC_OBJECT: u8 = b'O';
pub(crate) const BC_OBJECT_DIRECT: u8 = 0x60;
pub(crate) const OBJECT_DIRECT_MAX: usize = 0x0f;

pub(crate) const BC_CLASS: u8 = b'C';
