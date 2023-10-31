use std::io::Error as IoError;
use std::io::Write;

use super::Rc;
use crate::data::Flags;
use crate::data::Header;
use crate::data::Record;
use crate::data::SequenceType;

#[derive(Debug, Clone)]
struct WriteCounter<W: Write> {
    w: W,
    n: usize,
}

impl<W: Write> WriteCounter<W> {
    fn new(w: W) -> Self {
        Self { w, n: 0 }
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

fn write_variable_length<W: Write>(mut n: u64, mut w: W) -> Result<(), IoError> {
    let mut basis = 1;
    while basis * 128 <= n {
        basis *= 128;
    }

    while basis > 1 {
        w.write_all(&[((n / basis) | 0x80) as u8])?;
        n %= basis;
        basis /= 128;
    }

    w.write_all(&[n as u8])
}

fn write_length<W: Write>(mut l: u64, mut w: W) -> Result<(), IoError> {
    while l > (u32::MAX as u64) {
        w.write_all(&u32::MAX.to_le_bytes()[..])?;
        l -= u32::MAX as u64;
    }
    let n = l as u32;
    w.write_all(&n.to_le_bytes()[..])
}

pub struct Encoder<'z> {
    header: Header,

    ids: WriteCounter<zstd::Encoder<'z, Vec<u8>>>,
    seqs: WriteCounter<zstd::Encoder<'z, Vec<u8>>>,
    lens: WriteCounter<zstd::Encoder<'z, Vec<u8>>>,
}

impl Encoder<'_> {
    pub fn new(sequence_type: SequenceType) -> Self {
        let mut header = Header::default();
        header.sequence_type = sequence_type;
        header.flags = Flags::new(0x02 | 0x08 | 0x20); // sequence | lenghts | ids

        let mut ids = zstd::Encoder::new(Vec::new(), 0).unwrap();
        ids.include_magicbytes(false).unwrap();
        let mut seqs = zstd::Encoder::new(Vec::new(), 0).unwrap();
        seqs.include_magicbytes(false).unwrap();
        let mut lens = zstd::Encoder::new(Vec::new(), 0).unwrap();
        lens.include_magicbytes(false).unwrap();

        Encoder {
            header,
            ids: WriteCounter::new(ids),
            seqs: WriteCounter::new(seqs),
            lens: WriteCounter::new(lens),
        }
    }

    pub fn push(&mut self, record: &Record) -> Result<(), IoError> {
        if let Some(id) = record.id.as_ref() {
            self.ids.write_all(id.as_bytes())?;
            self.ids.write_all(b"\0")?;
        } else {
            panic!("missing ids")
        }

        if let Some(seq) = record.sequence.as_ref() {
            let length = seq.len();
            write_length(length as u64, &mut self.lens)?;
            self.seqs.write_all(seq.as_bytes())?;
            self.seqs.flush()?;
        } else {
            panic!("missing sequence")
        }

        self.header.number_of_sequences += 1;
        Ok(())
    }

    pub fn write<W: Write>(self, mut file: W) -> Result<(), IoError> {
        let og_ids = self.ids.n as u64;
        let og_lens = self.lens.n as u64;
        let og_seqs = self.seqs.n as u64;

        let ids_buffer = self.ids.w.finish()?;
        let lens_buffer = self.lens.w.finish()?;
        let seqs_buffer = self.seqs.w.finish()?;

        // --- header ---
        file.write_all(&[0x01, 0xF9, 0xEC])?; // format descriptor
        file.write_all(&[
            self.header.format_version as u8,
            self.header.sequence_type as u8,
            self.header.flags.into(),
            self.header.name_separator as u8,
        ])?;
        write_variable_length(self.header.line_length, &mut file)?;
        write_variable_length(self.header.number_of_sequences, &mut file)?;

        // -- ids ---
        write_variable_length(og_ids, &mut file)?;
        write_variable_length(ids_buffer.len() as u64, &mut file)?;
        file.write_all(ids_buffer.as_slice())?;

        // -- lengths --
        write_variable_length(og_lens, &mut file)?;
        write_variable_length(lens_buffer.len() as u64, &mut file)?;
        file.write_all(lens_buffer.as_slice())?;

        // -- seq --
        write_variable_length(og_seqs, &mut file)?;
        write_variable_length(seqs_buffer.len() as u64, &mut file)?;
        file.write_all(seqs_buffer.as_slice())?;

        file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder() {
        let mut encoder = Encoder::new(SequenceType::Protein);
        let mut r1 = Record {
            id: Some("r1".into()),
            sequence: Some("MYYK".into()),
            ..Default::default()
        };
        encoder.push(&r1).unwrap();

        let mut r2 = Record {
            id: Some("r2".into()),
            sequence: Some("MTTE".into()),
            ..Default::default()
        };
        encoder.push(&r2).unwrap();

        let f = std::fs::File::create("/tmp/test.naf").unwrap();
        encoder.write(f).unwrap();
    }
}
