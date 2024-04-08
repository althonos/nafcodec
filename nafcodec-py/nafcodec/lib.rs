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
use pyo3::types::PyList;
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

#[pymethods]
impl Record {
    #[new]
    #[pyo3(signature = (*, id=None, comment=None, sequence=None, quality=None, length=None))]
    fn __init__<'py>(
        py: Python<'py>,
        id: Option<Py<PyString>>,
        comment: Option<Py<PyString>>,
        sequence: Option<Py<PyString>>,
        quality: Option<Py<PyString>>,
        mut length: Option<u64>,
    ) -> PyResult<PyClassInitializer<Self>> {
        // Check lengths are consistent.
        if let Some(seq) = sequence.as_ref() {
            if let Some(qual) = quality.as_ref() {
                if seq.bind(py).len()? != qual.bind(py).len()? {
                    return Err(PyValueError::new_err(
                        "lengths of sequence and quality don't match",
                    ));
                }
            }
            if let Some(&l) = length.as_ref() {
                if seq.bind(py).len()? != l as usize {
                    return Err(PyValueError::new_err(
                        "length of sequence and record length don't match",
                    ));
                }
            } else {
                length = Some(seq.bind(py).len()? as u64);
            }
        }
        if let Some(qual) = quality.as_ref() {
            if let Some(&l) = length.as_ref() {
                if qual.bind(py).len()? != l as usize {
                    return Err(PyValueError::new_err(
                        "length of quality and record length don't match",
                    ));
                }
            } else {
                length = Some(qual.bind(py).len()? as u64);
            }
        }

        Ok(PyClassInitializer::from(Record {
            id,
            comment,
            sequence,
            quality,
            length,
        }))
    }

    fn __repr__<'py>(slf: PyRef<'py, Self>) -> PyResult<PyObject> {
        let py = slf.py();
        let format = pyo3::intern!(py, "format");
        let args = PyList::empty_bound(py);
        if let Some(id) = &slf.id {
            args.append(pyo3::intern!(py, "id={!r}").call_method1(format, (id,))?)?;
        }
        if let Some(comment) = &slf.comment {
            args.append(pyo3::intern!(py, "comment={!r}").call_method1(format, (comment,))?)?;
        }
        if let Some(sequence) = &slf.sequence {
            args.append(pyo3::intern!(py, "sequence={!r}").call_method1(format, (sequence,))?)?;
        }
        if let Some(quality) = &slf.quality {
            args.append(pyo3::intern!(py, "quality={!r}").call_method1(format, (quality,))?)?;
        }
        if let Some(length) = &slf.length {
            args.append(format!("length={}", length).to_object(py))?;
        }
        pyo3::intern!(py, "Record({})")
            .call_method1(
                format,
                (pyo3::intern!(py, ", ").call_method1("join", (args,))?,),
            )
            .map(|x| x.to_object(py))
    }
}

impl pyo3::conversion::IntoPy<Record> for nafcodec::Record {
    fn into_py(self, py: Python<'_>) -> Record {
        let id = self.id.map(|x| PyString::new_bound(py, &x).into());
        let sequence = self.sequence.map(|x| PyString::new_bound(py, &x).into());
        let comment = self.comment.map(|x| PyString::new_bound(py, &x).into());
        let quality = self.quality.map(|x| PyString::new_bound(py, &x).into());
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
    pub fn __init__<'py>(file: Bound<'py, PyAny>) -> PyResult<PyClassInitializer<Self>> {
        let py = file.py();
        let decoder = match PyFileRead::from_ref(&file) {
            Ok(handle) => {
                let wrapper = PyFileWrapper::PyFile(handle);
                nafcodec::Decoder::new(std::io::BufReader::new(wrapper))
                    .map_err(|e| convert_error(py, e, None))?
            }
            Err(_e) => {
                let path = py
                    .import_bound("os")?
                    .call_method1(pyo3::intern!(py, "fspath"), (file,))?
                    .extract::<Bound<'_, PyString>>()?;
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

    pub fn __iter__(slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        Ok(slf)
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<Record>> {
        let result = slf.deref_mut().decoder.next().transpose();
        let py = slf.py();
        match result {
            Ok(None) => Ok(None),
            Ok(Some(record)) => Ok(Some(record.into_py(py))),
            Err(e) => Err(convert_error(py, e, None)),
        }
    }

    /// `str`: The type of sequence stored in the archive.
    #[getter]
    pub fn sequence_type(slf: PyRef<'_, Self>) -> &Bound<'_, PyString> {
        use nafcodec::SequenceType;
        let py = slf.py();
        match slf.decoder.sequence_type() {
            SequenceType::Dna => pyo3::intern!(py, "dna"),
            SequenceType::Rna => pyo3::intern!(py, "rna"),
            SequenceType::Protein => pyo3::intern!(py, "protein"),
            SequenceType::Text => pyo3::intern!(py, "text"),
        }
    }

    /// `str`: The length of sequence lines in the original FASTA file.
    #[getter]
    pub fn format_version(slf: PyRef<'_, Self>) -> &Bound<'_, PyString> {
        use nafcodec::FormatVersion;
        let py = slf.py();
        match slf.decoder.header().format_version() {
            FormatVersion::V1 => pyo3::intern!(py, "v1"),
            FormatVersion::V2 => pyo3::intern!(py, "v2"),
        }
    }

    /// `int`: The length of sequence lines in the original FASTA file.
    #[getter]
    pub fn line_length(slf: PyRef<'_, Self>) -> u64 {
        slf.decoder.header().line_length()
    }

    /// `str`: The separator between sequence identifiers and comments.
    #[getter]
    pub fn name_separator(slf: PyRef<'_, Self>) -> char {
        slf.decoder.header().name_separator()
    }

    /// `int`: The total number of sequences stored in the archive.
    #[getter]
    pub fn number_of_sequences(slf: PyRef<'_, Self>) -> u64 {
        slf.decoder.header().number_of_sequences()
    }
}

/// An encoder/decoder for Nucleotide Archive Format files.
#[pymodule]
#[pyo3(name = "lib")]
pub fn init<'py>(_py: Python<'py>, m: &Bound<'py, PyModule>) -> PyResult<()> {
    m.add("__package__", "nafcodec")?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", env!("CARGO_PKG_AUTHORS").replace(':', "\n"))?;

    m.add_class::<Decoder>()?;
    m.add_class::<Record>()?;

    Ok(())
}
