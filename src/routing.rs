#[inline(always)]
pub fn extract_id(bytes: &[u8]) -> Option<u64> {
    // Quick check: does it start with "{"rid":"
    if !bytes.starts_with(b"{\"rid\":") {
        return None;
    }

    // Start parsing after `"id":`
    let mut i = 7; // skip {"rid":
    let start = i;

    // parse digits
    while let Some(&b) = bytes.get(i) {
        if !b.is_ascii_digit() {
            break;
        }
        i += 1;
    }

    std::str::from_utf8(&bytes[start..i]).ok()?.parse().ok()
}
