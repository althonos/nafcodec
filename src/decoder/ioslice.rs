use std::io::Error as IoError;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::rc::Rc;
use std::sync::RwLock;

pub struct IoSlice<R: Read + Seek> {
    reader: Rc<RwLock<R>>,
    pos: u64,
    start: u64,
    end: u64,
}

impl<R: Read + Seek> IoSlice<R> {
    pub fn new(reader: Rc<RwLock<R>>, start: u64, end: u64) -> Self {
        Self {
            reader,
            start,
            end,
            pos: start,
        }
    }
}

impl<R: Read + Seek> Read for IoSlice<R> {
    fn read(&mut self, mut buffer: &mut [u8]) -> Result<usize, IoError> {
        let mut reader = self.reader.write().unwrap();
        reader.seek(SeekFrom::Start(self.pos))?;

        let remaining = (self.end - self.pos) as usize;
        if buffer.len() > remaining {
            buffer = &mut buffer[..remaining];
        }

        let n = reader.read(buffer)?;
        self.pos += n as u64;

        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn read() {
        let data = b"ABCDEF";
        let cursor = Cursor::new(&data);
        let f = Rc::new(RwLock::new(cursor));

        let mut s1 = IoSlice::new(f.clone(), 0, 2);
        let mut s2 = IoSlice::new(f.clone(), 2, 4);
        let mut s3 = IoSlice::new(f.clone(), 4, 6);

        let mut x1 = String::new();
        let mut x2 = String::new();
        let mut x3 = String::new();

        s2.read_to_string(&mut x2);
        assert_eq!(x2, "CD");

        s1.read_to_string(&mut x1);
        assert_eq!(x1, "AB");

        s3.read_to_string(&mut x3);
        assert_eq!(x3, "EF");

        x2.clear();
        s2.read_to_string(&mut x2);
        assert_eq!(x2, "");
    }
}
