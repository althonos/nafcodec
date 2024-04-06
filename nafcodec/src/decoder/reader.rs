use std::ffi::CString;
use std::io::BufRead;

use crate::data::MaskUnit;
use crate::data::SequenceType;

// --- CStringReader -----------------------------------------------------------

#[derive(Debug)]
pub struct CStringReader<R: BufRead> {
    reader: R,
}

impl<R: BufRead> CStringReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: BufRead> Iterator for CStringReader<R> {
    type Item = Result<CString, std::io::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = Vec::new();
        match self.reader.read_until(0, &mut buffer) {
            Ok(0) => None,
            Err(e) => Some(Err(e)),
            Ok(_) => Some(Ok(CString::from_vec_with_nul(buffer)
                .expect("buffer should contain a single nul byte"))),
        }
    }
}

// --- LengthReader ------------------------------------------------------------

#[derive(Debug)]
pub struct LengthReader<R: BufRead> {
    reader: R,
}

impl<R: BufRead> LengthReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: BufRead> Iterator for LengthReader<R> {
    type Item = Result<u64, std::io::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut n = 0u64;
        let mut x = u32::MAX;
        let mut buffer = [0u8; 4];

        while x == u32::MAX {
            if let Err(e) = self.reader.read_exact(&mut buffer[..]) {
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => return None,
                    _ => return Some(Err(e)),
                }
            }
            x = nom::number::complete::le_u32::<&[u8], nom::error::Error<&[u8]>>(&buffer[..])
                .unwrap()
                .1;
            n += x as u64;
        }

        Some(Ok(n))
    }
}

// --- SequenceReader ----------------------------------------------------------

#[derive(Debug)]
pub struct SequenceReader<R: BufRead> {
    reader: R,
    ty: SequenceType,
    cache: Option<char>,
}

impl<R: BufRead> SequenceReader<R> {
    pub fn new(reader: R, ty: SequenceType) -> Self {
        Self {
            reader,
            ty,
            cache: None,
        }
    }

    pub fn next(&mut self, length: u64) -> Result<String, std::io::Error> {
        let l = length as usize;
        if self.ty.is_nucleotide() {
            let mut sequence = String::with_capacity(l);
            if self.cache.is_some() && l > 0 {
                sequence.push(self.cache.take().unwrap());
            }
            while sequence.len() < l {
                match self.ty {
                    SequenceType::Dna => self.read_nucleotide::<'T'>(l, &mut sequence)?,
                    SequenceType::Rna => self.read_nucleotide::<'U'>(l, &mut sequence)?,
                    _ => unreachable!(),
                }
            }
            Ok(sequence)
        } else {
            let mut sequence = Vec::with_capacity(l);
            while sequence.len() < l {
                self.read_text(l, &mut sequence)?;
            }
            String::from_utf8(sequence)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
    }

    fn read_text(&mut self, length: usize, sequence: &mut Vec<u8>) -> Result<(), std::io::Error> {
        let buffer = self.reader.fill_buf()?;
        let n_to_copy = buffer.len().min(length - sequence.len());
        sequence.extend_from_slice(&buffer[..n_to_copy]);
        self.reader.consume(n_to_copy);
        Ok(())
    }

    fn read_nucleotide<const T: char>(
        &mut self,
        length: usize,
        sequence: &mut String,
    ) -> Result<(), std::io::Error> {
        let buffer = self.reader.fill_buf()?;

        let rem = length as usize - sequence.len();
        let n = buffer.len().min(rem / 2);

        for x in buffer.iter().take(n) {
            let c1 = Self::decode::<T>(x & 0x0F);
            sequence.push(c1);
            let c2 = Self::decode::<T>(x >> 4);
            sequence.push(c2);
        }

        if n < buffer.len() && sequence.len() == length as usize - 1 {
            let c1 = Self::decode::<T>(buffer[n] & 0x0F);
            sequence.push(c1);
            let c2 = Self::decode::<T>(buffer[n] >> 4);
            self.cache = Some(c2);
            self.reader.consume(n + 1);
        } else {
            self.reader.consume(n);
        }

        Ok(())
    }

    #[inline]
    fn decode<const T: char>(c: u8) -> char {
        match c {
            0x00 => '-',
            0x01 => T,
            0x02 => 'G',
            0x03 => 'K',
            0x04 => 'C',
            0x05 => 'Y',
            0x06 => 'S',
            0x07 => 'B',
            0x08 => 'A',
            0x09 => 'W',
            0x0A => 'R',
            0x0B => 'D',
            0x0C => 'M',
            0x0D => 'H',
            0x0E => 'V',
            0x0F => 'N',
            _ => unreachable!(),
        }
    }
}

// --- MaskReader --------------------------------------------------------------

#[derive(Debug)]
pub struct MaskReader<R: BufRead> {
    reader: R,
    total: u64,
    current: u64,
    mask: bool,
}

impl<R: BufRead> MaskReader<R> {
    pub fn new(reader: R, total: u64) -> Self {
        Self {
            reader,
            total,
            current: 0,
            mask: false,
        }
    }
}

impl<R: BufRead> Iterator for MaskReader<R> {
    type Item = Result<MaskUnit, std::io::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }

        let mut n = 0u64;
        loop {
            let mut i = 0;
            let buf = match self.reader.fill_buf() {
                Err(e) => return Some(Err(e)),
                Ok(buf) if buf.len() == 0 => break,
                Ok(buf) => buf,
            };
            while i < buf.len() && buf[i] == 0xFF {
                n += 0xFF;
                i += 1;
            }
            if i < buf.len() {
                n += buf[i] as u64;
                self.reader.consume(i + 1);
                break;
            }
            self.reader.consume(i);
        }

        self.current += n;
        if self.mask {
            self.mask = false;
            Some(Ok(MaskUnit::Masked(n)))
        } else {
            self.mask = true;
            Some(Ok(MaskUnit::Unmasked(n)))
        }
    }
}
