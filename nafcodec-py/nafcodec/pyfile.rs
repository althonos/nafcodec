use std::fs::File;
use std::io::Error as IoError;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use pyo3::exceptions::PyOSError;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyByteArray;
use pyo3::types::PyBytes;
use pyo3::types::PyLong;
use pyo3::PyObject;

// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! transmute_file_error {
    ($self:ident, $e:ident, $msg:expr, $py:expr) => {{
        // Attempt to transmute the Python OSError to an actual
        // Rust `std::io::Error` using `from_raw_os_error`.
        if $e.is_instance_of::<PyOSError>($py) {
            if let Ok(code) = &$e.value($py).getattr("errno") {
                if let Ok(n) = code.extract::<i32>() {
                    return Err(IoError::from_raw_os_error(n));
                }
            }
        }

        // if the conversion is not possible for any reason we fail
        // silently, wrapping the Python error, and returning a
        // generic Rust error instead.
        $e.restore($py);
        Err(IoError::new(std::io::ErrorKind::Other, $msg))
    }};
}

// ---------------------------------------------------------------------------

/// A wrapper around a readable Python file borrowed within a GIL lifetime.
#[derive(Debug)]
pub struct PyFileRead {
    file: PyObject,
    has_readinto: bool,
}

impl PyFileRead {
    pub fn from_ref<'p>(file: &'p PyAny) -> PyResult<PyFileRead> {
        let py = file.py();

        let implementation = py
            .import(pyo3::intern!(py, "sys"))?
            .getattr(pyo3::intern!(py, "implementation"))?
            .getattr(pyo3::intern!(py, "name"))?;

        if file.hasattr(pyo3::intern!(py, "readinto"))?
            && implementation.eq(pyo3::intern!(py, "cpython"))?
        {
            let b = PyByteArray::new(py, &[]);
            if let Ok(res) = file.call_method1(pyo3::intern!(py, "readinto"), (b,)) {
                if res.downcast::<PyLong>().is_ok() {
                    return Ok({
                        PyFileRead {
                            file: file.to_object(py),
                            has_readinto: true,
                        }
                    });
                }
            }
        }

        let res = file.call_method1(pyo3::intern!(py, "read"), (0,))?;
        if res.downcast::<PyBytes>().is_ok() {
            Ok(PyFileRead {
                file: file.to_object(py),
                has_readinto: false,
            })
        } else {
            let ty = res.get_type().name()?.to_string();
            Err(PyTypeError::new_err(format!(
                "expected bytes, found {}",
                ty
            )))
        }
    }

    fn read_read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        Python::with_gil(|py| {
            match self
                .file
                .call_method1(py, pyo3::intern!(py, "read"), (buf.len(),))
            {
                Ok(obj) => {
                    // Check `fh.read` returned bytes, else raise a `TypeError`.
                    if let Ok(bytes) = obj.extract::<&PyBytes>(py) {
                        let b = bytes.as_bytes();
                        (&mut buf[..b.len()]).copy_from_slice(b);
                        Ok(b.len())
                    } else {
                        let ty = obj.as_ref(py).get_type().name()?.to_string();
                        let msg = format!("expected bytes, found {}", ty);
                        PyTypeError::new_err(msg).restore(py);
                        Err(IoError::new(
                            std::io::ErrorKind::Other,
                            "fh.read did not return bytes",
                        ))
                    }
                }
                Err(e) => {
                    transmute_file_error!(self, e, "read method failed", py)
                }
            }
        })
    }

    fn read_readinto(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        Python::with_gil(|py| {
            let memview = unsafe {
                let m = pyo3::ffi::PyMemoryView_FromMemory(
                    buf.as_mut_ptr() as *mut i8,
                    buf.len() as isize,
                    pyo3::ffi::PyBUF_WRITE,
                );
                PyObject::from_owned_ptr_or_err(py, m)?
            };
            match self
                .file
                .call_method1(py, pyo3::intern!(py, "readinto"), (memview,))
            {
                Ok(n) => match n.extract::<usize>(py) {
                    Ok(n) => Ok(n),
                    Err(e) => {
                        let ty = n.as_ref(py).get_type().name()?.to_string();
                        let msg = format!("expected int, found {}", ty);
                        PyTypeError::new_err(msg).restore(py);
                        Err(IoError::new(
                            std::io::ErrorKind::Other,
                            "fh.readinto did not return int",
                        ))
                    }
                },
                Err(e) => {
                    transmute_file_error!(self, e, "readinto method failed", py)
                }
            }
        })
    }
}

impl Read for PyFileRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        if self.has_readinto {
            self.read_readinto(buf)
        } else {
            self.read_read(buf)
        }
    }
}

impl Seek for PyFileRead {
    fn seek(&mut self, seek: SeekFrom) -> Result<u64, IoError> {
        let (offset, whence) = match seek {
            SeekFrom::Start(n) => (n as i64, 0),
            SeekFrom::Current(n) => (n, 1),
            SeekFrom::End(n) => (n, 2),
        };
        Python::with_gil(
            |py| match self.file.call_method1(py, "seek", (offset, whence)) {
                Ok(obj) => {
                    if let Ok(n) = obj.extract::<u64>(py) {
                        Ok(n)
                    } else {
                        let ty = obj.as_ref(py).get_type().name()?.to_string();
                        let msg = format!("expected int, found {}", ty);
                        PyTypeError::new_err(msg).restore(py);
                        Err(IoError::new(
                            std::io::ErrorKind::Other,
                            "fh.seek did not return position",
                        ))
                    }
                }
                Err(e) => Err(IoError::new(std::io::ErrorKind::Unsupported, e.to_string())),
            },
        )
    }
}

// ---------------------------------------------------------------------------

pub enum PyFileWrapper {
    PyFile(PyFileRead),
    File(File),
}

impl Read for PyFileWrapper {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        match self {
            PyFileWrapper::PyFile(r) => r.read(buf),
            PyFileWrapper::File(f) => f.read(buf),
        }
    }
}

impl Seek for PyFileWrapper {
    fn seek(&mut self, seek: SeekFrom) -> Result<u64, IoError> {
        match self {
            PyFileWrapper::PyFile(r) => r.seek(seek),
            PyFileWrapper::File(f) => f.seek(seek),
        }
    }
}
