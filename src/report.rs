//! Error reports: what the system looked like when an error reached the
//! host, before the stacks were emptied, so that a person or an AI agent can
//! see what went wrong without running the code again.

use crate::*;

/// The state at an error, as `Throw::context` gives it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ErrorContext {
    /// The data stack at the error, the top last. A word that took its
    /// arguments before it failed (such as `/` on division by zero) has
    /// taken them.
    pub stack: Vec<Cell>,
    /// The float stack at the error, the top last.
    pub fstack: Vec<f64>,
    /// The words that were running, from the outermost to the innermost,
    /// such as `MAIN`, `STEP`, `/`. Empty when no word ran, as for an
    /// undefined word.
    pub words: Vec<String>,
    /// The input the error came from: `input` (the text the host gave),
    /// `evaluate` (a string given to EVALUATE), the path of a file, or
    /// `block n`.
    pub source: String,
    /// The line in that input, from 1; for a block, the line of 64
    /// characters in it.
    pub line: usize,
    /// The column of `token` in that line, in characters from 1; 0 when
    /// no word was read.
    pub column: usize,
    /// The last word read from the input: the word that failed, or the word
    /// that ran the code that failed.
    pub token: String,
}

impl Forth {
    /// Gives an error its context, unless it has one; `first` is the line
    /// of the terminal source before the host's text began.
    pub(crate) fn report(&self, mut e: Throw, first: usize) -> Throw {
        if e.context.is_none() {
            e.context = Some(Box::new(self.error_context(first)));
        }
        e
    }

    fn error_context(&self, first: usize) -> ErrorContext {
        let s = self.sources.last().unwrap();
        let bytes = &self.mem[s.buf..s.buf + s.len];
        let to_in = usize::try_from(self.get(IN_ADDR)).unwrap_or(0).min(bytes.len());
        // The last word read ends just before >IN, past the blank after it.
        let end = bytes[..to_in].iter().rposition(|&b| b > b' ').map_or(0, |i| i + 1);
        let start = bytes[..end].iter().rposition(|&b| b <= b' ').map_or(0, |i| i + 1);
        let token = String::from_utf8_lossy(&bytes[start..end]).into_owned();
        let (source, line, line_start) = match &s.origin {
            Origin::Terminal => ("input".to_string(), s.line - first, 0),
            Origin::Evaluate => {
                let lines = bytes[..start].iter().filter(|&&b| b == b'\n').count();
                let line_start =
                    bytes[..start].iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
                ("evaluate".to_string(), lines + 1, line_start)
            }
            Origin::File { fid, .. } => {
                let name =
                    self.open_file(*fid).map_or_else(|| "file".to_string(), |f| f.name.clone());
                (name, s.line, 0)
            }
            Origin::Block => (format!("block {}", s.line), start / 64 + 1, start / 64 * 64),
        };
        let column = if token.is_empty() {
            0
        } else {
            String::from_utf8_lossy(&bytes[line_start..start]).chars().count() + 1
        };
        ErrorContext {
            stack: self.stack.clone(),
            fstack: self.fstack.clone(),
            words: self.running_words(&token),
            source,
            line,
            column,
            token,
        }
    }

    /// The words that were running: the word of each frame, then the word
    /// the innermost frame was calling. Without frames, the word read last,
    /// if it is one.
    fn running_words(&self, token: &str) -> Vec<String> {
        let name = |xt: usize| match self.dict.get(xt) {
            Some(w) if w.name.is_empty() => ":NONAME".to_string(),
            Some(w) => w.name.clone(),
            None => "?".to_string(),
        };
        let Some(top) = self.frames.last() else {
            return self.find(token).map(name).into_iter().collect();
        };
        let calling = match top.pc.checked_sub(1).and_then(|pc| top.code.get(pc)) {
            Some(Instr::Call(xt)) => Some(*xt),
            _ => None,
        };
        let xts = self.frames.iter().map(|f| f.xt).chain(calling);
        // A word of the system (such as `/`, defined in Forth) is where the
        // chain ends: how it works inside is not the program's business.
        let mut words = Vec::new();
        for xt in xts {
            words.push(name(xt));
            if xt < self.fence {
                break;
            }
        }
        words
    }
}
