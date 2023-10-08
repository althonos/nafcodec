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
    /// `str` or `None`: The record identifier.
    #[pyo3(get, set)]
    id: Option<Py<PyString>>,
    /// `str` or `None`: The record comment.
    #[pyo3(get, set)]
    comment: Option<Py<PyString>>,
    /// `str` or `None`: The record sequence.
    #[pyo3(get, set)]
    sequence: Option<Py<PyString>>,
    /// `str` or `None`: The record quality.
    #[pyo3(get, set)]
    quality: Option<Py<PyString>>,
    /// `str` or `None`: The record sequence length.
    #[pyo3(get, set)]
    length: Option<u64>,
}

impl pyo3::conversion::IntoPy<Record> for nafcodec::data::Record {
    fn into_py(self, py: Python<'_>) -> Record {
        let id = self.id.map(|x| PyString::new(py, &x).into());
        let sequence = self.sequence.map(|x| PyString::new(py, &x).into());
        let comment = self.comment.map(|x| PyString::new(py, &x).into());
        let quality = self.quality.map(|x| PyString::new(py, &x).into());
        let length = self.length;
        Record {
            id,
            sequence,
            comment,
            quality,
            length,
        }
    }
}

#[pyclass(module = "nafcodec.lib")]
pub struct Decoder {
    decoder: nafcodec::Decoder<'static, BufReader<File>>,
}

#[pymethods]
impl Decoder {
    #[new]
    fn __init__(path: &PyAny) -> PyResult<PyClassInitializer<Self>> {
        let py = path.py();
        let fspath = py
            .import("os")?
            .call_method1(pyo3::intern!(py, "fspath"), (path,))?
            .downcast::<PyString>()?;
        let mut decoder = nafcodec::Decoder::from_path(fspath.to_str()?).unwrap();
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
