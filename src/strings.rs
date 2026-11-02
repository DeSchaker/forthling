//! Dynamic strings and string arrays: the word list DYNAMIC-STRINGS, installed as the
//! extension `Strings`. The words are written in Forth (strings.fs); only
//! `$TMP`, which needs the output of the system, is in Rust.

use crate::*;

/// The extension: `forth.install(&Strings)`; ENVIRONMENT? knows it as
/// FORTHLING-DYNAMIC-STRINGS.
pub struct Strings;

/// The block of the last `$TMP`, which the next `$TMP` frees.
#[derive(Default)]
struct Tmp(Option<usize>);

impl Extension for Strings {
    fn name(&self) -> &'static str {
        "FORTHLING-DYNAMIC-STRINGS"
    }

    fn install(&self, f: &mut Forth) {
        let wid = f.wordlist("DYNAMIC-STRINGS");
        f.add_word(wid, "$TMP", tmp).expect("DYNAMIC-STRINGS is a word list");
        // Compile strings.fs into the word list, and leave the search order
        // and the compilation word list as they were.
        let (order, current) = (f.order.clone(), f.current_wid);
        f.interpret("ALSO DYNAMIC-STRINGS DEFINITIONS").expect("DYNAMIC-STRINGS is a word list");
        let result = f.interpret(include_str!("strings.fs"));
        f.order = order;
        f.current_wid = current;
        result.expect("strings.fs must compile");
    }
}

/// `$TMP ( xt -- c-addr u )`: runs xt with its output caught. The text is
/// valid until the next `$TMP`.
fn tmp(f: &mut Forth) -> Res {
    let xt = f.pop()?;
    let xt = usize::try_from(xt).ok().filter(|&i| i < f.dict.len()).ok_or_else(|| {
        Throw::with(INVALID_ARGUMENT, format!("$TMP: invalid execution token {xt}"))
    })?;
    if let Some(old) = f.state::<Tmp>().0.take() {
        f.heap.free(old);
    }
    let start = f.out.len();
    let ran = f.execute(xt);
    let text = f.out.split_off(start.min(f.out.len()));
    ran?;
    let a =
        f.heap.allocate(text.len(), f.here).ok_or_else(|| Throw::with(ALLOCATE_FAILED, "$TMP"))?;
    f.mem[a..a + text.len()].copy_from_slice(&text);
    f.state::<Tmp>().0 = Some(a);
    f.push(to_cell(a));
    f.push(to_cell(text.len()));
    Ok(())
}
