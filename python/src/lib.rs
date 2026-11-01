//! The Python module `forthling`: a Forth system as a Python object, built
//! on the host API of the core.
//!
//! ```python
//! import forthling
//! f = forthling.Forth()
//! f.interpret(": SQUARE DUP * ; 7 SQUARE .")   # returns "49 "
//! f.add_word("HYPOT", math.hypot, inputs=2)     # pushes a float
//! ```

// Lints of clippy::pedantic that the code is clean for; `clippy -D warnings` in CI
// keeps it so. (The rest of pedantic is not enabled: see the open issues.)
#![warn(
    clippy::cast_lossless,
    clippy::doc_markdown,
    clippy::float_cmp,
    clippy::format_push_string,
    clippy::many_single_char_names,
    clippy::match_same_arms,
    clippy::must_use_candidate,
    clippy::redundant_closure_for_method_calls,
    clippy::return_self_not_must_use,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::struct_excessive_bools,
    clippy::unnecessary_literal_bound,
    clippy::uninlined_format_args
)]

use forthling::{
    Cell, Extension, Forth, Limits, MAX_MEMORY, MIN_MEMORY, MemoryFs, Permission, Throw,
};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyFloat, PyInt, PyTuple};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::time::Duration;

create_exception!(
    forthling,
    ForthError,
    PyException,
    "A Forth exception: `code` is the THROW code, `output` what was printed before it."
);

/// The THROW code of a Python exception in a word added with `add_word`.
const PYTHON_EXCEPTION: Cell = -257;

/// Makes "FORTHLING-PYTHON" known to ENVIRONMENT?.
struct PythonHost;

impl Extension for PythonHost {
    fn name(&self) -> &'static str {
        "FORTHLING-PYTHON"
    }

    fn install(&self, _: &mut Forth) {}
}

/// A Forth system.
#[pyclass(unsendable, name = "Forth", module = "forthling")]
struct PyForth {
    forth: Forth,
    // The Python exception that made a word throw PYTHON_EXCEPTION, to raise
    // again when the Forth code does not catch it.
    pending: Rc<RefCell<Option<PyErr>>>,
    // The files of the sandbox, in memory; None outside it, where the files
    // are those of the disk.
    files: Option<MemoryFs>,
}

/// A value a Python function gives back to Forth.
enum Value {
    Cell(Cell),
    Float(f64),
}

/// The values of a result: None gives none, a bool a flag, an int a cell, a
/// float a float, and a tuple or list each of its items in turn.
fn values(v: &Bound<'_, PyAny>, out: &mut Vec<Value>) -> PyResult<()> {
    if v.is_none() {
    } else if v.is_instance_of::<PyBool>() {
        out.push(Value::Cell(if v.is_truthy()? { -1 } else { 0 }));
    } else if v.is_instance_of::<PyInt>() {
        out.push(Value::Cell(v.extract()?));
    } else if v.is_instance_of::<PyFloat>() {
        out.push(Value::Float(v.extract()?));
    } else if v.is_instance_of::<PyTuple>() || v.is_instance_of::<pyo3::types::PyList>() {
        for item in v.try_iter()? {
            values(&item?, out)?;
        }
    } else {
        let kind = v.get_type().name()?;
        return Err(PyTypeError::new_err(format!("a word cannot return a {kind}")));
    }
    Ok(())
}

impl PyForth {
    /// The files in memory, which only the sandbox has.
    fn memory_files(&self) -> PyResult<&MemoryFs> {
        self.files.as_ref().ok_or_else(|| {
            PyValueError::new_err("only a sandbox (sandbox=True) has files in memory")
        })
    }

    /// The Python exception for a Forth exception.
    fn error(&self, py: Python<'_>, e: Throw, output: String) -> PyErr {
        if e.code == PYTHON_EXCEPTION {
            if let Some(err) = self.pending.borrow_mut().take() {
                return err;
            }
        }
        let err = ForthError::new_err(e.to_string());
        let value = err.value(py);
        value.setattr("code", e.code).ok();
        value.setattr("output", output).ok();
        value.setattr("hint", e.hint()).ok();
        // Where it happened and the stacks then; None for an error that did
        // not come through interpret or include, such as pop() on an empty stack.
        let c = e.context();
        value.setattr("stack", c.map(|c| c.stack.clone())).ok();
        value.setattr("fstack", c.map(|c| c.fstack.clone())).ok();
        value.setattr("words", c.map(|c| c.words.clone())).ok();
        value.setattr("source", c.map(|c| c.source.clone())).ok();
        value.setattr("line", c.map(|c| c.line)).ok();
        value.setattr("column", c.map(|c| c.column)).ok();
        value.setattr("token", c.map(|c| c.token.clone())).ok();
        err
    }

    /// Runs `run` and returns the output it produced.
    fn run(
        &mut self,
        py: Python<'_>,
        run: impl FnOnce(&mut Forth) -> forthling::Res,
    ) -> PyResult<String> {
        self.pending.borrow_mut().take();
        let result = run(&mut self.forth);
        let output = String::from_utf8_lossy(&self.forth.take_output()).into_owned();
        match result {
            Ok(()) => Ok(output),
            Err(e) => Err(self.error(py, e, output)),
        }
    }
}

#[pymethods]
impl PyForth {
    /// A new system with `memory` bytes of memory (1 MiB by default).
    /// `input` is what KEY, ACCEPT and REFILL read; there is none by default.
    /// `sandbox=True` forbids files and limits each evaluation to 10 million
    /// steps, 5 seconds and 1 MiB of output, and the space of definitions and
    /// the like to 4 MiB; `max_steps`, `max_time` (in seconds), `max_output`
    /// and `max_space` (in bytes) set a limit of their own. `allow_hosts` is
    /// a list of hosts that HTTP requests of the Forth code may go to (`ALSO
    /// HTTP`); without it no request goes out, and `sandbox=True` closes the
    /// network altogether unless there are hosts.
    #[new]
    #[pyo3(signature = (
        memory = None, input = "", sandbox = false,
        max_steps = None, max_time = None, max_output = None, max_space = None,
        allow_hosts = None
    ))]
    #[allow(clippy::too_many_arguments)] // the keyword arguments of the Python constructor
    fn new(
        memory: Option<usize>,
        input: &str,
        sandbox: bool,
        max_steps: Option<u64>,
        max_time: Option<f64>,
        max_output: Option<usize>,
        max_space: Option<usize>,
        allow_hosts: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let mut forth = match memory {
            None => Forth::new(),
            Some(n) if (MIN_MEMORY..=MAX_MEMORY).contains(&n) && n % 8 == 0 => {
                Forth::with_memory(n)
            }
            Some(n) => {
                return Err(PyValueError::new_err(format!(
                    "memory must be a multiple of 8 between {MIN_MEMORY} and {MAX_MEMORY}, not {n}"
                )));
            }
        };
        forth.set_input(Box::new(io::Cursor::new(input.as_bytes().to_vec())));
        let mut limits = if sandbox { Limits::SANDBOX } else { Limits::default() };
        limits.steps = max_steps.or(limits.steps);
        limits.output = max_output.or(limits.output);
        limits.space = max_space.or(limits.space);
        if let Some(t) = max_time {
            let time = Duration::try_from_secs_f64(t)
                .map_err(|_| PyValueError::new_err(format!("max_time must be seconds, not {t}")))?;
            limits.time = Some(time);
        }
        forth.set_limits(limits);
        // In the sandbox the files are in memory, not on the disk.
        let files = sandbox.then(|| MemoryFs::with_limit(MemoryFs::SANDBOX_LIMIT));
        if let Some(files) = &files {
            forth.set_file_system(Box::new(files.clone())).expect("no block buffers in use yet");
        }
        forth.install(&forthling::Json);
        forth.install(&forthling_regex::Regex);
        let hosts = allow_hosts.unwrap_or_default();
        forth.install(&forthling_http::Http::new().allow_all(hosts.iter().map(String::as_str)));
        if sandbox && hosts.is_empty() {
            forth.permit(Permission::Network, false);
        }
        forth.install(&PythonHost);
        Ok(PyForth { forth, pending: Rc::default(), files })
    }

    /// Interprets Forth source and returns what it printed. Raises
    /// `ForthError` for an uncaught THROW, which also empties the stacks.
    fn interpret(&mut self, py: Python<'_>, text: &str) -> PyResult<String> {
        self.run(py, |f| f.interpret(text))
    }

    /// Runs a Forth source file and returns what it printed. In the sandbox
    /// the file is read from the disk and put in the files in memory first,
    /// so that the Forth code sees it there.
    fn include(&mut self, py: Python<'_>, path: &str) -> PyResult<String> {
        if let (Some(files), Ok(bytes)) = (&self.files, std::fs::read(path)) {
            files.insert(path, bytes).map_err(|e| PyValueError::new_err(format!("{path}: {e}")))?;
        }
        self.run(py, |f| f.include(path))
    }

    /// Puts a file in the sandbox's files, replacing one of the same name;
    /// `data` is str (written as UTF-8) or bytes.
    fn write_file(&self, name: &str, data: &Bound<'_, PyAny>) -> PyResult<()> {
        let bytes: Vec<u8> = match data.extract::<String>() {
            Ok(text) => text.into_bytes(),
            Err(_) => data.extract()?,
        };
        let files = self.memory_files()?;
        files.insert(name, bytes).map_err(|e| PyValueError::new_err(format!("{name}: {e}")))
    }

    /// The contents of a file of the sandbox, as bytes, or None when there
    /// is no such file.
    fn read_file<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Option<Bound<'py, PyBytes>>> {
        Ok(self.memory_files()?.get(name).map(|bytes| PyBytes::new(py, &bytes)))
    }

    /// Removes a file of the sandbox; False when there was none.
    fn delete_file(&self, name: &str) -> PyResult<bool> {
        Ok(self.memory_files()?.remove(name).is_ok())
    }

    /// The names of the sandbox's files, sorted.
    fn files(&self) -> PyResult<Vec<String>> {
        Ok(self.memory_files()?.paths())
    }

    /// Adds a word that calls `func` with `inputs` cells from the data
    /// stack, the deepest first. What it returns goes to the stacks: None
    /// nothing, a bool a flag, an int a cell, a float to the float stack, a
    /// tuple or list each item in turn. The word goes into the word list
    /// `wordlist` (made if needed, then found with ALSO name), by default
    /// FORTH-WORDLIST. A Python exception in `func` is THROW -257 in
    /// Forth; if nothing catches it, `interpret` raises it again.
    #[pyo3(signature = (name, func, inputs = 0, wordlist = None))]
    fn add_word(
        &mut self,
        py: Python<'_>,
        name: &str,
        func: Py<PyAny>,
        inputs: usize,
        wordlist: Option<&str>,
    ) -> PyResult<()> {
        let wid = wordlist.map_or(1, |w| self.forth.wordlist(w));
        let pending = Rc::clone(&self.pending);
        let word = move |f: &mut Forth| {
            let mut args = vec![0; inputs];
            for arg in args.iter_mut().rev() {
                *arg = f.pop()?;
            }
            let result = Python::attach(|py| {
                let mut out = Vec::new();
                values(&func.bind(py).call1(PyTuple::new(py, args)?)?, &mut out)?;
                Ok::<_, PyErr>(out)
            });
            match result {
                Ok(out) => {
                    for v in out {
                        match v {
                            Value::Cell(n) => f.push(n),
                            Value::Float(r) => f.fpush(r)?,
                        }
                    }
                    Ok(())
                }
                Err(err) => {
                    let detail = err.to_string();
                    *pending.borrow_mut() = Some(err);
                    Err(Throw::with(PYTHON_EXCEPTION, detail))
                }
            }
        };
        self.forth.add_word(wid, name, word).map_err(|e| self.error(py, e, String::new()))
    }

    /// Pushes a cell on the data stack.
    fn push(&mut self, n: Cell) {
        self.forth.push(n);
    }

    /// Pops a cell from the data stack.
    fn pop(&mut self, py: Python<'_>) -> PyResult<Cell> {
        self.forth.pop().map_err(|e| self.error(py, e, String::new()))
    }

    /// Pushes a float on the float stack.
    fn fpush(&mut self, py: Python<'_>, r: f64) -> PyResult<()> {
        self.forth.fpush(r).map_err(|e| self.error(py, e, String::new()))
    }

    /// Pops a float from the float stack.
    fn fpop(&mut self, py: Python<'_>) -> PyResult<f64> {
        self.forth.fpop().map_err(|e| self.error(py, e, String::new()))
    }

    /// The data stack as a list, the top last.
    #[getter]
    fn stack(&self) -> Vec<Cell> {
        self.forth.stack().to_vec()
    }

    /// Whether BYE was executed.
    #[getter]
    fn finished(&self) -> bool {
        self.forth.finished()
    }
}

#[pymodule]
#[pyo3(name = "forthling")]
fn forthling_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyForth>()?;
    m.add("ForthError", m.py().get_type::<ForthError>())?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
