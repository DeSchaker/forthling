//! The Python module `forthling`: a Forth system as a Python object, built
//! on the host API of the core.
//!
//! ```python
//! import forthling
//! f = forthling.Forth()
//! f.interpret(": SQUARE DUP * ; 7 SQUARE .")   # returns "49 "
//! f.add_word("HYPOT", math.hypot, inputs=2)     # pushes a float
//! ```

use forthling::{Cell, Extension, Forth, Throw, MAX_MEMORY, MIN_MEMORY};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyFloat, PyInt, PyTuple};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

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
    fn name(&self) -> &str {
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
    #[new]
    #[pyo3(signature = (memory = None, input = ""))]
    fn new(memory: Option<usize>, input: &str) -> PyResult<Self> {
        let mut forth = match memory {
            None => Forth::new(),
            Some(n) if (MIN_MEMORY..=MAX_MEMORY).contains(&n) && n % 8 == 0 => {
                Forth::with_memory(n)
            }
            Some(n) => {
                return Err(PyValueError::new_err(format!(
                    "memory must be a multiple of 8 between {MIN_MEMORY} and {MAX_MEMORY}, not {n}"
                )))
            }
        };
        forth.set_input(Box::new(io::Cursor::new(input.as_bytes().to_vec())));
        forth.install(&PythonHost);
        Ok(PyForth { forth, pending: Rc::default() })
    }

    /// Interprets Forth source and returns what it printed. Raises
    /// ForthError for an uncaught THROW, which also empties the stacks.
    fn interpret(&mut self, py: Python<'_>, text: &str) -> PyResult<String> {
        self.run(py, |f| f.interpret(text))
    }

    /// Runs a Forth source file and returns what it printed.
    fn include(&mut self, py: Python<'_>, path: &str) -> PyResult<String> {
        self.run(py, |f| f.include(path))
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
