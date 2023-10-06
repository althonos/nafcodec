use std::ffi::CString;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use nom::IResult;

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

    pub fn next(&mut self) -> Result<CString, std::io::Error> {
        let mut buffer = Vec::new();
        self.reader.read_until(0, &mut buffer)?;
        Ok(CString::from_vec_with_nul(buffer).expect("buffer should contain a single nul byte"))
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

    pub fn next(&mut self) -> Result<u64, std::io::Error> {
        let mut n = 0u64;
        let mut x = u32::MAX;
        let mut buffer = [0u8; 4];

        while x == u32::MAX {
            self.reader.read_exact(&mut buffer[..])?;
            x = nom::number::complete::le_u32::<&[u8], nom::error::Error<&[u8]>>(&buffer[..])
                .unwrap()
                .1;
            n += x as u64;
        }

        Ok(n)
    }
}

// --- TextReader --------------------------------------------------------------

#[derive(Debug)]
pub struct TextReader<R: BufRead> {
    reader: R,
}

impl<R: BufRead> TextReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn next(&mut self, length: u64) -> Result<Vec<u8>, std::io::Error> {
        let mut buffer = vec![0; length as usize];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
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
        if self.ty != SequenceType::Dna {
            unimplemented!()
        }

        let mut sequence = String::with_capacity(length as usize);

        if self.cache.is_some() && length > 0 {
            sequence.push(self.cache.take().unwrap());
        }

        while (sequence.len() as u64) < length {
            let mut i = 0;
            let buffer = self.reader.fill_buf()?;

            while i < buffer.len() && (sequence.len() as u64) < length {
                let c1 = Self::decode(buffer[i] & 0x0F);
                sequence.push(c1);

                let c2 = Self::decode(buffer[i] >> 4);
                if sequence.len() as u64 == length {
                    self.cache = Some(c2);
                } else {
                    sequence.push(c2);
                }

                i += 1;
            }

            self.reader.consume(i);
        }

        Ok(sequence)
    }

    pub fn decode(c: u8) -> char {
        match c {
            0x00 => '-',
            0x01 => 'T',
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
