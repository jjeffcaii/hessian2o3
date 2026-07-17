use base64::{Engine as _, alphabet, engine};
use std::time;

#[inline]
pub(crate) fn encode_base64(b: &[u8]) -> String {
    const G: engine::GeneralPurpose =
        engine::GeneralPurpose::new(&alphabet::STANDARD, engine::general_purpose::PAD);

    let mut buf = String::with_capacity(b.len() * 3 / 4);
    G.encode_string(b, &mut buf);
    buf
}

#[inline]
pub(crate) fn millis_to_system_time(millis: i64) -> time::SystemTime {
    if millis >= 0 {
        time::SystemTime::UNIX_EPOCH + time::Duration::from_millis(millis as u64)
    } else {
        // process timestamp before 1970
        time::UNIX_EPOCH - time::Duration::from_millis(millis.unsigned_abs())
    }
}
