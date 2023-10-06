use std::io::BufReader;
use std::io::Cursor;
use std::io::Error as IoError;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

pub trait Tee: Read + Sized {
    fn tee(&self) -> Result<Self, IoError>;
}

impl<T> Tee for Cursor<T>
where
    T: AsRef<[u8]> + Clone,
{
    fn tee(&self) -> Result<Self, IoError> {
        let pos = self.position();
        let mut copy = Cursor::new(self.get_ref().clone());
        copy.set_position(pos);
        Ok(copy)
    }
}

impl<T> Tee for BufReader<T>
where
    T: Tee + Seek,
{
    fn tee(&self) -> Result<Self, IoError> {
        let mut copy = self.get_ref().tee()?;
        copy.seek(SeekFrom::Current(-(self.buffer().len() as i64)))?;
        Ok(BufReader::new(copy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor() {
        let data = b"ABCDEF";

        let mut c1 = Cursor::new(&data);
        let mut b1 = [0; 2];

        c1.read_exact(&mut b1).unwrap();
        assert_eq!(&b1, b"AB");

        let mut c2 = c1.tee().unwrap();
        let mut b2 = [0; 2];

        c1.read_exact(&mut b1).unwrap();
        assert_eq!(&b1, b"CD");
        c2.read_exact(&mut b2).unwrap();
        assert_eq!(&b2, b"CD");
    }

    #[test]
    fn bufreader() {
        let data = b"ABCDEF";

        let mut c1 = BufReader::new(Cursor::new(&data));
        let mut b1 = [0; 2];

        c1.read_exact(&mut b1).unwrap();
        assert_eq!(&b1, b"AB");

        let mut c2 = c1.tee().unwrap();
        let mut b2 = [0; 2];

        c1.read_exact(&mut b1).unwrap();
        assert_eq!(&b1, b"CD");
        c2.read_exact(&mut b2).unwrap();
        assert_eq!(&b2, b"CD");
    }
}
