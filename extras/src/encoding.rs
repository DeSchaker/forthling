//! The word list ENCODING: base64 and base64url (RFC 4648), hex, SHA-256
//! (FIPS 180-4) and CRC-32 (the polynomial of zip and PNG), written by hand.

use crate::{EXTRAS_ERROR, word};
use forthling::{Cell, Forth, Res, Throw};

const LIST: &str = "ENCODING";

fn error(what: &str) -> Throw {
    Throw::with(EXTRAS_ERROR, what.to_string())
}

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64_encode(data: &[u8], alphabet: &[u8; 64], pad: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        // A chunk of k bytes gives k + 1 characters; padding makes it four.
        for i in 0..=chunk.len() {
            out.push(alphabet[((n >> (18 - 6 * i)) & 63) as usize]);
        }
        if pad {
            out.resize(out.len() + 3 - chunk.len(), b'=');
        }
    }
    out
}

fn base64_decode(text: &[u8], alphabet: &[u8; 64], padding_required: bool) -> Res<Vec<u8>> {
    let mut data = text;
    let mut padded = 0;
    while let [rest @ .., b'='] = data {
        data = rest;
        padded += 1;
    }
    if padded > 2 || data.len() % 4 == 1 {
        return Err(error("base64 text with a wrong length or padding"));
    }
    if (padded > 0 || padding_required) && (data.len() + padded) % 4 != 0 {
        return Err(error("base64 text with a wrong padding"));
    }
    let mut out = Vec::with_capacity(data.len() / 4 * 3 + 2);
    let (mut acc, mut bits) = (0u32, 0);
    for &c in data {
        let Some(v) = alphabet.iter().position(|&a| a == c).and_then(|v| u8::try_from(v).ok())
        else {
            return Err(error("a character that is not in the base64 alphabet"));
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from(acc >> bits).expect("eight bits"));
            acc &= (1 << bits) - 1;
        }
    }
    // The bits left over must be zero, so that one text has one meaning.
    if acc != 0 {
        return Err(error("base64 text whose last character has bits that are not zero"));
    }
    Ok(out)
}

fn hex_decode(text: &[u8]) -> Res<Vec<u8>> {
    if text.len() % 2 != 0 {
        return Err(error("hex text with an odd number of digits"));
    }
    let digit = |c: u8| {
        (c as char).to_digit(16).ok_or_else(|| error("a character that is not a hex digit"))
    };
    text.chunks(2)
        .map(|p| Ok(u8::try_from((digit(p[0])? << 4) | digit(p[1])?).expect("two digits")))
        .collect()
}

const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

fn compress(h: &mut [u32; 8], block: &[u8]) {
    let mut w = [0u32; 64];
    for (i, word) in block.chunks(4).enumerate() {
        w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let mut v = *h;
    for i in 0..64 {
        let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
        let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
        let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
        let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
        let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
        let t2 = s0.wrapping_add(maj);
        v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
    }
    for (x, y) in h.iter_mut().zip(v) {
        *x = x.wrapping_add(y);
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    // The text, a one bit, zeros, and the length in bits as 64 bits: whole blocks of 64 bytes.
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&(data.len() as u64 * 8).to_be_bytes());
    for block in message.chunks(64) {
        compress(&mut h, block);
    }
    let mut out = [0u8; 32];
    for (bytes, x) in out.chunks_mut(4).zip(h) {
        bytes.copy_from_slice(&x.to_be_bytes());
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A word `( c-addr u -- c-addr' u' )` that works on the bytes of a text; the
/// work counts by the length of the text and the new text is `ALLOCATE`d.
fn transform(f: &mut Forth, name: &str, work: impl Fn(&[u8]) -> Res<Vec<u8>> + 'static) {
    word(f, LIST, name, move |f| {
        let text = f.pop_bytes()?;
        f.charge(text.len())?;
        f.push_bytes(&work(&text)?)
    });
}

pub fn install(f: &mut Forth) {
    transform(f, ">BASE64", |t| Ok(base64_encode(t, STANDARD, true)));
    transform(f, "BASE64>", |t| base64_decode(t, STANDARD, true));
    transform(f, ">BASE64URL", |t| Ok(base64_encode(t, URL, false)));
    transform(f, "BASE64URL>", |t| base64_decode(t, URL, false));
    transform(f, ">HEX", |t| Ok(t.iter().flat_map(|b| format!("{b:02x}").into_bytes()).collect()));
    transform(f, "HEX>", hex_decode);
    transform(f, "SHA-256", |t| Ok(sha256(t).to_vec()));
    word(f, LIST, "CRC-32", |f| {
        let text = f.pop_bytes()?;
        f.charge(text.len())?;
        f.push(Cell::from(crc32(&text)));
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write;
        bytes.iter().fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").unwrap();
            s
        })
    }

    #[test]
    fn sha256_known_values() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // 55, 56 and 64 bytes are where the padding changes the number of blocks.
        assert_eq!(
            hex(&sha256(&[b'a'; 56])),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
    }

    #[test]
    fn crc32_known_values() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"The quick brown fox jumps over the lazy dog"), 0x414F_A339);
    }

    #[test]
    fn base64_rfc4648_vectors() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64_encode(plain.as_bytes(), STANDARD, true), coded.as_bytes());
            assert_eq!(base64_decode(coded.as_bytes(), STANDARD, true).unwrap(), plain.as_bytes());
        }
    }

    #[test]
    fn base64url_has_no_padding_and_reads_both() {
        let data = [0xfb, 0xff, 0xfe];
        assert_eq!(base64_encode(&data, STANDARD, true), b"+//+");
        assert_eq!(base64_encode(&data, URL, false), b"-__-");
        assert_eq!(base64_encode(b"fo", URL, false), b"Zm8");
        assert_eq!(base64_decode(b"Zm8", URL, false).unwrap(), b"fo");
        assert_eq!(base64_decode(b"Zm8=", URL, false).unwrap(), b"fo");
    }

    #[test]
    fn bad_base64_is_refused() {
        for text in ["Zg", "Zg=", "Z", "Zm9v=", "Zm9*", "Zh==", "=Zg=", "Zg==Zg=="] {
            assert!(base64_decode(text.as_bytes(), STANDARD, true).is_err(), "{text}");
        }
    }
}
