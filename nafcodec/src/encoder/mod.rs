use std::fs::File;
use std::io::Error as IoError;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

mod counter;
mod storage;

use self::counter::WriteCounter;
pub use self::storage::Memory;
pub use self::storage::Storage;

use super::Rc;
use crate::data::Flags;
use crate::data::Header;
use crate::data::Record;
use crate::data::SequenceType;

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

pub struct Encoder<'z, S: Storage> {
    header: Header,
    storage: S,

    ids: WriteCounter<zstd::Encoder<'z, S::Buffer>>,
    // com: WriteCounter<zstd::Encoder<'z, S::Buffer>>,
    len: WriteCounter<zstd::Encoder<'z, S::Buffer>>,
    seq: WriteCounter<zstd::Encoder<'z, S::Buffer>>,
    // qual: WriteCounter<zstd::Encoder<'z, S::Buffer>>,
    // mask: WriteCounter<zstd::Encoder<'z, S::Buffer>>,
}

impl Encoder<'_, Memory> {
    /// Create a new encoder for the given sequence type using memory buffers.
    ///
    /// Use `Encoder::with_storage` to specificy a different storage type,
    /// such as a `TempDir` to store the temporary compressed blocks into
    /// a temporary directory.
    pub fn new(sequence_type: SequenceType) -> Result<Self, IoError> {
        Self::with_storage(sequence_type, Default::default())
    }
}

impl<S: Storage> Encoder<'_, S> {
    pub fn with_storage(sequence_type: SequenceType, storage: S) -> Result<Self, IoError> {
        let mut header = Header::default();
        header.sequence_type = sequence_type;
        header.flags = Flags::new(0x02 | 0x08 | 0x20); // sequence | lenghts | ids

        let mut ids = zstd::Encoder::new(storage.create_buffer()?, 0).unwrap();
        ids.include_magicbytes(false).unwrap();
        let mut seqs = zstd::Encoder::new(storage.create_buffer()?, 0).unwrap();
        seqs.include_magicbytes(false).unwrap();
        let mut lens = zstd::Encoder::new(storage.create_buffer()?, 0).unwrap();
        lens.include_magicbytes(false).unwrap();

        Ok(Encoder {
            header,
            storage,
            ids: WriteCounter::new(ids),
            seq: WriteCounter::new(seqs),
            len: WriteCounter::new(lens),
        })
    }
}

impl<S: Storage> Encoder<'_, S> {
    pub fn push(&mut self, record: &Record) -> Result<(), IoError> {
        if let Some(id) = record.id.as_ref() {
            self.ids.write_all(id.as_bytes())?;
            self.ids.write_all(b"\0")?;
        } else {
            panic!("missing ids")
        }

        if let Some(seq) = record.sequence.as_ref() {
            let length = seq.len();
            write_length(length as u64, &mut self.len)?;
            self.seq.write_all(seq.as_bytes())?;
            self.seq.flush()?;
        } else {
            panic!("missing sequence")
        }

        self.header.number_of_sequences += 1;
        Ok(())
    }

    pub fn write<W: Write>(self, mut file: W) -> Result<(), IoError> {
        let og_ids = self.ids.len() as u64;
        let og_lens = self.len.len() as u64;
        let og_seqs = self.seq.len() as u64;

        let mut ids_buffer = self.ids.into_inner().finish()?;
        ids_buffer.flush()?;
        let mut len_buffer = self.len.into_inner().finish()?;
        len_buffer.flush()?;
        let mut seq_buffer = self.seq.into_inner().finish()?;
        seq_buffer.flush()?;

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
        write_variable_length(self.storage.buffer_length(&ids_buffer)? as u64, &mut file)?;
        self.storage.write_buffer(ids_buffer, &mut file)?;

        // -- lengths --
        write_variable_length(og_lens, &mut file)?;
        write_variable_length(self.storage.buffer_length(&len_buffer)? as u64, &mut file)?;
        self.storage.write_buffer(len_buffer, &mut file)?;

        // -- seq --
        write_variable_length(og_seqs, &mut file)?;
        write_variable_length(self.storage.buffer_length(&seq_buffer)? as u64, &mut file)?;
        self.storage.write_buffer(seq_buffer, &mut file)?;

        file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_memory() {
        let mut encoder = Encoder::<Memory>::new(SequenceType::Protein).unwrap();
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

        let f = std::fs::File::create("/tmp/test1.naf").unwrap();
        encoder.write(f).unwrap();
    }

    #[cfg(feature = "tempfile")]
    #[test]
    fn encoder_tempfile() {
        let tempdir = tempfile::TempDir::new().unwrap();
        let mut encoder = Encoder::with_storage(SequenceType::Protein, tempdir).unwrap();
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

        let f = std::fs::File::create("/tmp/test2.naf").unwrap();
        encoder.write(f).unwrap();
    }
}
