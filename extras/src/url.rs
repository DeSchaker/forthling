//! The word list URL: percent-encoding (RFC 3986) and splitting a URL into
//! scheme, user information, host, port, path, query and fragment. The words
//! only cut text; they do not check that the characters of a part are allowed.

use crate::{EXTRAS_ERROR, word};
use forthling::{Forth, Res, Throw, to_cell};
use std::ops::Range;

const LIST: &str = "URL";

fn error(what: &str) -> Throw {
    Throw::with(EXTRAS_ERROR, what.to_string())
}

/// A byte that percent-encoding leaves alone: the unreserved characters.
fn unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
}

fn percent_encode(text: &[u8]) -> Vec<u8> {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = Vec::with_capacity(text.len());
    for &b in text {
        if unreserved(b) {
            out.push(b);
        } else {
            out.extend_from_slice(&[
                b'%',
                DIGITS[usize::from(b >> 4)],
                DIGITS[usize::from(b & 15)],
            ]);
        }
    }
    out
}

fn percent_decode(text: &[u8]) -> Res<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i] == b'%' {
            let digit = |k: usize| text.get(k).and_then(|&c| (c as char).to_digit(16));
            let (Some(hi), Some(lo)) = (digit(i + 1), digit(i + 2)) else {
                return Err(error("a % that is not followed by two hex digits"));
            };
            out.push(u8::try_from(hi * 16 + lo).expect("two hex digits"));
            i += 3;
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    Ok(out)
}

/// Where the parts of a URL are in its text; a part that is not there is empty.
#[derive(Default)]
struct Parts {
    scheme: Range<usize>,
    userinfo: Range<usize>,
    host: Range<usize>,
    port: Option<u16>,
    path: Range<usize>,
    query: Range<usize>,
    fragment: Range<usize>,
}

fn position(text: &[u8], from: usize, to: usize, byte: u8) -> Option<usize> {
    text[from..to].iter().position(|&b| b == byte).map(|i| from + i)
}

/// Cuts a URL or a relative reference: the parts of RFC 3986, section 3.
fn split(text: &[u8]) -> Res<Parts> {
    let mut parts = Parts::default();
    let mut end = text.len();
    if let Some(i) = position(text, 0, end, b'#') {
        parts.fragment = i + 1..end;
        end = i;
    }
    if let Some(i) = position(text, 0, end, b'?') {
        parts.query = i + 1..end;
        end = i;
    }
    // A scheme is a letter and then letters, digits, + - . up to the first colon.
    let mut start = 0;
    if text.first().is_some_and(u8::is_ascii_alphabetic) {
        let n = text[..end]
            .iter()
            .position(|&b| !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')))
            .unwrap_or(end);
        if n < end && text[n] == b':' {
            parts.scheme = 0..n;
            start = n + 1;
        }
    }
    if text[start..end].starts_with(b"//") {
        let from = start + 2;
        let to = position(text, from, end, b'/').unwrap_or(end);
        parts.path = to..end;
        authority(text, from, to, &mut parts)?;
    } else {
        parts.path = start..end;
    }
    Ok(parts)
}

/// Cuts the authority at `from..to`: `userinfo@host:port`. The host of an
/// IPv6 address keeps its brackets.
fn authority(text: &[u8], mut from: usize, to: usize, parts: &mut Parts) -> Res {
    if let Some(i) = text[from..to].iter().rposition(|&b| b == b'@') {
        parts.userinfo = from..from + i;
        from += i + 1;
    }
    let host_end;
    let mut port_from = to;
    if text.get(from) == Some(&b'[') {
        let Some(close) = position(text, from, to, b']') else {
            return Err(error("a host with [ and no ]"));
        };
        host_end = close + 1;
        if host_end < to {
            if text[host_end] != b':' {
                return Err(error("text after the ] of a host"));
            }
            port_from = host_end + 1;
        }
    } else if let Some(i) = position(text, from, to, b':') {
        host_end = i;
        port_from = i + 1;
    } else {
        host_end = to;
    }
    parts.host = from..host_end;
    let digits = &text[port_from.min(to)..to];
    if !digits.is_empty() {
        let port = digits
            .iter()
            .try_fold(0u32, |n, &b| {
                if b.is_ascii_digit() {
                    Some((n * 10 + u32::from(b - b'0')).min(65_536))
                } else {
                    None
                }
            })
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(|| error("a port that is not a number from 0 to 65535"))?;
        parts.port = Some(port);
    }
    Ok(())
}

/// Pops a text ( c-addr u ), counts its length as work, and cuts it as a URL.
fn pop_url(f: &mut Forth) -> Res<(usize, Parts)> {
    let (a, n) = f.pop_str()?;
    f.charge(n)?;
    let parts = split(&f.memory()[a..a + n])?;
    Ok((a, parts))
}

/// A word `( c-addr u -- c-addr' u' )` that gives one part, in the place where it is.
fn part_word(f: &mut Forth, name: &str, part: fn(&Parts) -> Range<usize>) {
    word(f, LIST, name, move |f| {
        let (a, parts) = pop_url(f)?;
        let range = part(&parts);
        f.push(to_cell(a + range.start));
        f.push(to_cell(range.len()));
        Ok(())
    });
}

/// The value of the first parameter called `name` in a query: `a=1&b=2`.
/// A name without `=` has an empty value. Names are compared as they are written.
fn param(query: &[u8], name: &[u8]) -> Option<Range<usize>> {
    let mut at = 0;
    for pair in query.split(|&b| b == b'&') {
        let eq = pair.iter().position(|&b| b == b'=');
        if &pair[..eq.unwrap_or(pair.len())] == name {
            let start = at + eq.map_or(pair.len(), |i| i + 1);
            return Some(start..at + pair.len());
        }
        at += pair.len() + 1;
    }
    None
}

pub fn install(f: &mut Forth) {
    // >PERCENT ( c-addr u -- c-addr' u' ): every byte but A-Z a-z 0-9 - . _ ~
    // as %XX; the new text is ALLOCATEd.
    word(f, LIST, ">PERCENT", |f| {
        let text = f.pop_bytes()?;
        f.charge(text.len())?;
        f.push_bytes(&percent_encode(&text))
    });
    // PERCENT> ( c-addr u -- c-addr' u' ): %XX as a byte; a + stays a plus; the
    // new text is ALLOCATEd.
    word(f, LIST, "PERCENT>", |f| {
        let text = f.pop_bytes()?;
        f.charge(text.len())?;
        f.push_bytes(&percent_decode(&text)?)
    });
    part_word(f, "URL-SCHEME", |p| p.scheme.clone());
    part_word(f, "URL-USERINFO", |p| p.userinfo.clone());
    part_word(f, "URL-HOST", |p| p.host.clone());
    part_word(f, "URL-PATH", |p| p.path.clone());
    part_word(f, "URL-QUERY", |p| p.query.clone());
    part_word(f, "URL-FRAGMENT", |p| p.fragment.clone());
    // URL-PORT ( c-addr u -- n ): the port, or -1 when there is none.
    word(f, LIST, "URL-PORT", |f| {
        let (_, parts) = pop_url(f)?;
        f.push(parts.port.map_or(-1, i64::from));
        Ok(())
    });
    // URL-PARAM ( query-addr query-u name-addr name-u -- value-addr value-u true | false )
    word(f, LIST, "URL-PARAM", |f| {
        let (na, nn) = f.pop_str()?;
        let (qa, qn) = f.pop_str()?;
        f.charge(nn + qn)?;
        let found = param(&f.memory()[qa..qa + qn], &f.memory()[na..na + nn]);
        match found {
            Some(range) => {
                f.push(to_cell(qa + range.start));
                f.push(to_cell(range.len()));
                f.push(-1);
            }
            None => f.push(0),
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut(url: &str) -> [String; 7] {
        let p = split(url.as_bytes()).unwrap();
        let s = |r: &Range<usize>| url[r.clone()].to_string();
        [
            s(&p.scheme),
            s(&p.userinfo),
            s(&p.host),
            p.port.map_or(String::new(), |n| n.to_string()),
            s(&p.path),
            s(&p.query),
            s(&p.fragment),
        ]
    }

    #[test]
    fn splits_a_full_url() {
        assert_eq!(
            cut("https://user:pw@example.com:8080/a/b.html?x=1&y=2#top"),
            ["https", "user:pw", "example.com", "8080", "/a/b.html", "x=1&y=2", "top"]
        );
    }

    #[test]
    fn parts_may_be_missing() {
        assert_eq!(cut("http://example.com"), ["http", "", "example.com", "", "", "", ""]);
        assert_eq!(cut("http://example.com?q=1"), ["http", "", "example.com", "", "", "q=1", ""]);
        assert_eq!(cut("http://example.com:/p"), ["http", "", "example.com", "", "/p", "", ""]);
        assert_eq!(cut("mailto:a@b.nl"), ["mailto", "", "", "", "a@b.nl", "", ""]);
        assert_eq!(cut("/a/b?c#d"), ["", "", "", "", "/a/b", "c", "d"]);
        assert_eq!(cut("//host/p"), ["", "", "host", "", "/p", "", ""]);
        assert_eq!(cut("a:b:c"), ["a", "", "", "", "b:c", "", ""]);
        assert_eq!(cut("1a:b"), ["", "", "", "", "1a:b", "", ""]);
        assert_eq!(cut("dir/a:b"), ["", "", "", "", "dir/a:b", "", ""]);
        assert_eq!(cut(""), ["", "", "", "", "", "", ""]);
        // The first ? and # count, and a # hides a ?.
        assert_eq!(cut("http://h/p#f?x"), ["http", "", "h", "", "/p", "", "f?x"]);
        assert_eq!(cut("http://h/p?a?b=c"), ["http", "", "h", "", "/p", "a?b=c", ""]);
    }

    #[test]
    fn ipv6_hosts_keep_their_brackets() {
        assert_eq!(cut("http://[::1]:80/x"), ["http", "", "[::1]", "80", "/x", "", ""]);
        assert_eq!(cut("http://[2001:db8::7]/x"), ["http", "", "[2001:db8::7]", "", "/x", "", ""]);
    }

    #[test]
    fn the_last_at_ends_the_user_information() {
        assert_eq!(cut("ftp://a@b@host/"), ["ftp", "a@b", "host", "", "/", "", ""]);
    }

    #[test]
    fn bad_authorities_are_refused() {
        for url in [
            "http://host:80a/",
            "http://host:65536/",
            "http://host:-1/",
            "http://host:1:2/",
            "http://[::1/",
            "http://[::1]x/",
            "http://host:99999999999999999999/",
        ] {
            assert!(split(url.as_bytes()).is_err(), "{url}");
        }
        assert_eq!(cut("http://host:65535/")[3], "65535");
        assert_eq!(cut("http://host:0/")[3], "0");
    }

    #[test]
    fn percent_encoding_round_trips() {
        assert_eq!(percent_encode("a b&c=d/é~-._".as_bytes()), b"a%20b%26c%3Dd%2F%C3%A9~-._");
        assert_eq!(percent_encode(b""), b"");
        assert_eq!(percent_decode(b"a%20b%26c%3dd%2F%C3%A9+").unwrap(), "a b&c=d/é+".as_bytes());
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(percent_decode(&percent_encode(&all)).unwrap(), all);
    }

    #[test]
    fn bad_percent_text_is_refused() {
        for text in ["%", "%2", "%zz", "a%2", "%%20", "%2g"] {
            assert!(percent_decode(text.as_bytes()).is_err(), "{text}");
        }
    }

    #[test]
    fn finds_parameters() {
        let q = b"a=1&b=&c&a=2&d=x=y";
        assert_eq!(&q[param(q, b"a").unwrap()], b"1");
        assert_eq!(&q[param(q, b"b").unwrap()], b"");
        assert_eq!(&q[param(q, b"c").unwrap()], b"");
        assert_eq!(&q[param(q, b"d").unwrap()], b"x=y");
        assert!(param(q, b"e").is_none());
        assert!(param(q, b"A").is_none());
        assert!(param(b"", b"a").is_none());
        assert!(param(q, b"").is_none());
        assert_eq!(param(b"a=1&&b", b""), Some(4..4));
    }
}
