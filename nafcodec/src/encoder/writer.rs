use cfg_if::cfg_if;
use std::io::Error as IoError;
use std::io::Write;

use crate::data::SequenceType;
#[cfg(feature="lut")]
const MINIBYTE_LUT: [u8;32] = [
    0, 8, 7, 4, 11, 0, 0, 2,13,
    0, 0, 3,  0,12,15, 0, 0,
    0,10, 6,  1, 1,14, 9, 0,
    5, 0,  0, 0, 0, 0, 0];

pub struct SequenceWriter<W: Write> {
    ty: SequenceType,
    writer: W,
    cache: Option<u8>,
}

impl<W: Write> SequenceWriter<W> {
    pub fn new(ty: SequenceType, writer: W) -> Self {
        Self {
            writer,
            ty,
            cache: None,
        }
    }

    pub fn into_inner(mut self) -> Result<W, IoError> {
        // make sure to write the last letter of the last sequence if any.
        if let Some(letter) = self.cache.take() {
            cfg_if!{
                if #[cfg(features="lut")] {
                    self.writer.write_all(&[self.encode_lut(letter)?])?;
                } else {
                    self.writer.write_all(&[self.encode(letter)?])?;
                }
            }
        }
        self.writer.flush()?;
        Ok(self.writer)
    }

    #[cfg(feature="lut")]
    #[inline]
    fn encode_lut(&self, c: u8) -> Result<u8,IoError> {
            Ok(MINIBYTE_LUT[usize::from(c&(30|c>>6))])
    }

    #[inline]
    fn encode(&self, c: u8) -> Result<u8, IoError> {
        match c {
            b'A' => Ok(0x08),
            b'C' => Ok(0x04),
            b'G' => Ok(0x02),
            b'T' if self.ty == SequenceType::Dna => Ok(0x01),
            b'U' if self.ty == SequenceType::Rna => Ok(0x01),
            b'R' => Ok(0x0A),
            b'Y' => Ok(0x05),
            b'S' => Ok(0x06),
            b'W' => Ok(0x09),
            b'K' => Ok(0x03),
            b'M' => Ok(0x0C),
            b'B' => Ok(0x07),
            b'D' => Ok(0x0B),
            b'H' => Ok(0x0D),
            b'V' => Ok(0x0E),
            b'N' => Ok(0x0F),
            b'-' => Ok(0x00),
            _ => Err(IoError::new(
                std::io::ErrorKind::InvalidData,
                "unexpected sequence character",
            )),
        }
    }
}

impl<W: Write> Write for SequenceWriter<W> {
    fn write(&mut self, s: &[u8]) -> std::io::Result<usize> {
        let length = s.len();
        if length == 0 {
            return Ok(0);
        }

        if self.ty == SequenceType::Protein || self.ty == SequenceType::Text {
            return self.writer.write_all(s).map(|_| s.len());
        }

        let mut bytes = s;
        let mut encoded = Vec::with_capacity((s.len() + 1) / 2);
        if let Some(letter) = self.cache.take() {
            cfg_if!{
                if #[cfg(feature="lut")] {
                    let c = (self.encode_lut(s[0])? << 4) | self.encode(letter)?;
                } else {
                    let c = (self.encode(s[0])? << 4) | self.encode(letter)?;
                }
            }
            encoded.push(c);
            bytes = &s[1..];
        }

        for chunk in bytes.chunks(2) {
            if chunk.len() == 1 {
                assert!(self.cache.is_none());
                self.cache = Some(chunk[0]);
            } else {
                cfg_if!{
                    if #[cfg(feature="lut")] {
                        let c = (self.encode_lut(chunk[1])? << 4) | self.encode_lut(chunk[0])?;
                    } else {
                        let c = (self.encode(chunk[1])? << 4) | self.encode(chunk[0])?;
                    }
                }
                encoded.push(c);
            }
        }

        self.writer.write_all(&encoded)?;
        self.writer.flush()?;
        Ok(encoded.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
