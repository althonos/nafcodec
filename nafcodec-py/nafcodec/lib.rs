#![doc = include_str!("../README.md")]

extern crate nafcodec;
extern crate pyo3;

mod pyfile;
use self::pyfile::PyFileRead;
use self::pyfile::PyFileWrapper;

use std::io::BufReader;
use std::ops::DerefMut;

use pyo3::exceptions::PyFileNotFoundError;
use pyo3::exceptions::PyIsADirectoryError;
use pyo3::exceptions::PyOSError;
use pyo3::exceptions::PyUnicodeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;

/// Convert a `nafcodec::error::Error` into a Python exception.
fn convert_error(_py: Python, error: nafcodec::error::Error, path: Option<&str>) -> PyErr {
    use nafcodec::error::Error;

    match error {
        Error::Utf8(_utf8_error) => PyUnicodeError::new_err("failed to decode UTF-8 data"),
        Error::Nom(nom_error) => {
            PyValueError::new_err(format!("parser failed: {:?}", nom_error.code))
        }
        Error::Io(io_error) => {
            let desc = io_error.to_string();
            if let Some(p) = path.map(str::to_string) {
                match io_error.raw_os_error() {
                    Some(2) => PyFileNotFoundError::new_err((p,)),
                    #[cfg(target_os = "windows")]
                    Some(3) => PyFileNotFoundError::new_err((p,)),
                    #[cfg(not(target_os = "windows"))]
                    Some(21) => PyIsADirectoryError::new_err((p,)),
                    Some(code) => PyOSError::new_err((code, desc, p)),
                    None => PyOSError::new_err((desc,)),
                }
            } else {
                match io_error.raw_os_error() {
                    Some(2) => PyFileNotFoundError::new_err((desc,)),
                    #[cfg(target_os = "windows")]
                    Some(3) => PyFileNotFoundError::new_err((desc,)),
                    #[cfg(not(target_os = "windows"))]
                    Some(21) => PyIsADirectoryError::new_err((desc,)),
                    Some(code) => PyOSError::new_err((code, desc)),
                    None => PyOSError::new_err((desc,)),
                }
            }
        }
    }
}

/// A single sequence record stored in a Nucleotide Archive Format file.
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

impl pyo3::conversion::IntoPy<Record> for nafcodec::Record {
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

/// A streaming decoder to read a Nucleotide Archive Format file.
#[pyclass(module = "nafcodec.lib")]
pub struct Decoder {
    decoder: nafcodec::Decoder<'static, BufReader<PyFileWrapper>>,
}

#[pymethods]
impl Decoder {
    #[new]
    fn __init__(file: &PyAny) -> PyResult<PyClassInitializer<Self>> {
        let py = file.py();
        let decoder = match PyFileRead::from_ref(file) {
            Ok(handle) => {
                let wrapper = PyFileWrapper::PyFile(handle);
                nafcodec::Decoder::new(std::io::BufReader::new(wrapper))
                    .map_err(|e| convert_error(py, e, None))?
            }
            Err(_e) => {
                let path = py
                    .import("os")?
                    .call_method1(pyo3::intern!(py, "fspath"), (file,))?
                    .downcast::<PyString>()?;
                let path_str = path.to_str()?;
                let reader = std::fs::File::open(path_str)
                    .map_err(nafcodec::error::Error::Io)
                    .map_err(|e| convert_error(py, e, Some(path_str)))
                    .map(PyFileWrapper::File)?;
                nafcodec::Decoder::new(std::io::BufReader::new(reader))
                    .map_err(|e| convert_error(py, e, Some(path_str)))?
            }
        };

        Ok(Decoder { decoder }.into())
    }

    fn __iter__(slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        Ok(slf)
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<Record>> {
        let result = slf.deref_mut().decoder.next().transpose();
        let py = slf.py();
        match result {
            Ok(None) => Ok(None),
            Ok(Some(record)) => Ok(Some(record.into_py(py))),
            Err(e) => Err(convert_error(py, e, None)),
        }
    }

    #[getter]
    fn sequence_type(slf: PyRef<'_, Self>) -> &str {
        use nafcodec::SequenceType;
        match slf.decoder.sequence_type() {
            SequenceType::Dna => "dna",
            SequenceType::Rna => "rna",
            SequenceType::Protein => "protein",
            SequenceType::Text => "text",
        }
    }
}

/// An encoder/decoder for Nucleotide Archive Format files.
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
