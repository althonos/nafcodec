#![doc = include_str!("../README.md")]

extern crate nafcodec;
extern crate pyo3;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyString;
use std::fs::File;
use std::io::BufReader;
use std::ops::DerefMut;

#[pyclass(module = "nafcodec.lib")]
#[derive(Clone, Debug)]
pub struct Record {
    #[pyo3(get)]
    id: PyObject,
    #[pyo3(get)]
    sequence: PyObject,
}

impl pyo3::conversion::IntoPy<Record> for nafcodec::data::Record {
    fn into_py(self, py: Python<'_>) -> Record {
        let id = match self.id {
            Some(id) => PyString::new(py, &id).to_object(py),
            None => py.None(),
        };
        let sequence = match self.sequence {
            Some(seq) => PyString::new(py, &seq).to_object(py),
            None => py.None(),
        };
        Record { id, sequence }
    }
}

#[pyclass(module = "nafcodec.lib")]
pub struct Decoder {
    decoder: nafcodec::Decoder<'static, BufReader<File>>,
}

#[pymethods]
impl Decoder {
    #[new]
    fn __init__(path: &PyString) -> PyResult<PyClassInitializer<Self>> {
        let mut decoder = nafcodec::Decoder::from_path(path.to_str()?).unwrap();
        Ok(Decoder { decoder }.into())
    }

    fn __iter__(slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        Ok(slf)
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<Record>> {
        match slf.deref_mut().decoder.next() {
            None => Ok(None),
            Some(Ok(record)) => Ok(Some(Python::with_gil(|py| record.into_py(py)))),
            Some(Err(e)) => Err(PyRuntimeError::new_err("iterator failed")), // TODO
        }
    }
}

// impl From<nafcodec::Record> for Record {

// }

#[pymethods]
impl Record {}

/// PyO3 bindings to ``nafcodec``, an encoder/decoder for NAF files.
#[pymodule]
#[pyo3(name = "lib")]
pub fn init(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__package__", "nafcodec")?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", env!("CARGO_PKG_AUTHORS").replace(':', "\n"))?;

    m.add_class::<Decoder>()?;
    m.add_class::<Record>()?;

    Ok(())
}
