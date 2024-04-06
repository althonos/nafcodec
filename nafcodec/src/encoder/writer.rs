use std::io::Error as IoError;
use std::io::Write;

use crate::data::SequenceType;

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

    pub fn write_sequence<S: AsRef<str>>(&mut self, sequence: S) -> Result<(), IoError> {
        let s = sequence.as_ref();
        if s.len() == 0 {
            return Ok(());
        }

        if self.ty == SequenceType::Protein || self.ty == SequenceType::Text {
            return self.writer.write_all(s.as_bytes());
        }

        let mut bytes = s.as_bytes();
        let mut encoded = Vec::with_capacity((s.len() + 1) / 2);
        if let Some(letter) = self.cache.take() {
            let c = (Self::encode(bytes[0]) << 4) | (Self::encode(letter) << 4);
            encoded.push(c);
            bytes = &bytes[1..];
        }

        for chunk in bytes.chunks(2) {
            if chunk.len() == 1 {
                self.cache = Some(chunk[0]);
            } else {
                let c = (Self::encode(chunk[1]) << 4) | Self::encode(chunk[0]);
                encoded.push(c);
            }
        }

        println!("{:?}", s);
        println!("{:?}", encoded);
        self.writer.write_all(&encoded)?;
        self.writer.flush()
    }

    pub fn as_inner(&self) -> &W {
        &self.writer
    }

    pub fn as_inner_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    pub fn into_inner(mut self) -> Result<W, IoError> {
        // make sure to write the last letter of the last sequence if any.
        if let Some(letter) = self.cache.take() {
            self.writer.write_all(&[Self::encode(letter)])?;
        }
        self.writer.flush()?;
        Ok(self.writer)
    }

    #[inline]
    fn encode(c: u8) -> u8 {
        match c {
            b'A' => 0x08,
            b'C' => 0x04,
            b'G' => 0x02,
            b'T' => 0x01,
            b'U' => 0x01,
            b'R' => 0x0A,
            b'Y' => 0x05,
            b'S' => 0x06,
            b'W' => 0x09,
            b'K' => 0x03,
            b'M' => 0x0C,
            b'B' => 0x07,
            b'D' => 0x0B,
            b'H' => 0x0D,
            b'V' => 0x0E,
            b'N' => 0x0F,
            b'-' => 0x00,
            _ => panic!("FIXME: invalid sequence character"),
        }
    }
}
