use std::io::Error as IoError;
use std::io::Write;

/// A wrapper type to count the number of bytes written to a writer.
#[derive(Debug, Clone)]
pub struct WriteCounter<W: Write> {
    w: W,
    n: usize,
}

impl<W: Write> WriteCounter<W> {
    pub fn new(w: W) -> Self {
        Self { w, n: 0 }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn into_inner(self) -> W {
        self.w
    }
}

impl<W: Write> Write for WriteCounter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        match self.w.write(buf) {
            Err(e) => Err(e),
            Ok(n) => {
                self.n += n;
                Ok(n)
            }
        }
    }

    fn flush(&mut self) -> Result<(), IoError> {
        self.w.flush()
    }
}
