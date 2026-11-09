//! The word list UUID: a random version 4 UUID (RFC 9562), from the generator
//! behind `RANDOM`, so `SEED!` makes a run repeatable.

use crate::word;
use forthling::Forth;

const LIST: &str = "UUID";

/// The text `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx` of 16 random bytes: the
/// version bits are 0100 and the variant bits 10, the rest is random.
fn format(mut bytes: [u8; 16]) -> Vec<u8> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let mut out = Vec::with_capacity(36);
    for (i, b) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push(b'-');
        }
        out.push(DIGITS[usize::from(b >> 4)]);
        out.push(DIGITS[usize::from(b & 15)]);
    }
    out
}

pub fn install(f: &mut Forth) {
    // UUID4 ( -- c-addr u ): a random UUID as 36 characters; the text is ALLOCATEd.
    word(f, LIST, "UUID4", |f| {
        f.charge(36)?;
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&f.next_random().to_be_bytes());
        bytes[8..].copy_from_slice(&f.next_random().to_be_bytes());
        f.push_bytes(&format(bytes))
    });
}

#[cfg(test)]
mod tests {
    use super::format;

    #[test]
    fn shape() {
        assert_eq!(format([0; 16]), b"00000000-0000-4000-8000-000000000000");
        assert_eq!(format([255; 16]), b"ffffffff-ffff-4fff-bfff-ffffffffffff");
    }
}
