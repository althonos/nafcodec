use std::io::Error as IoError;
use std::io::Write;

mod counter;
mod storage;
mod writer;

use self::counter::WriteCounter;
pub use self::storage::Memory;
pub use self::storage::Storage;
use self::writer::SequenceWriter;

use crate::data::Flag;
use crate::data::Flags;
use crate::data::Header;
use crate::data::Record;
use crate::data::SequenceType;
use crate::FormatVersion;

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

/// A builder to configure and initialize an [`Encoder`].
#[derive(Debug, Clone)]
pub struct EncoderBuilder {
    sequence_type: SequenceType,
    id: bool,
    sequence: bool,
    quality: bool,
    comment: bool,
    compression_level: i32,
}

impl EncoderBuilder {
    /// Create a new encoder builder with default parameters.
    pub fn new(sequence_type: SequenceType) -> Self {
        Self {
            sequence_type,
            id: true,
            quality: false,
            comment: false,
            sequence: true,
            compression_level: 0,
        }
    }

    /// Create a new encoder builder from the given flags.
    ///
    /// This constructor can be used as a shortcut to setup encoding
    /// of a subset of supported fields. For instance, to write only
    /// the sequence identifiers and quality lines to an archive:
    /// ```
    /// # use nafcodec::{EncoderBuilder, Flag, Memory, SequenceType::Dna};
    /// let mut encoder = EncoderBuilder::from_flags(Dna, Flag::Id | Flag::Quality)
    ///     .with_memory()
    ///     .unwrap();
    /// # drop(encoder);
    /// ```
    pub fn from_flags<F: Into<Flags>>(sequence_type: SequenceType, flags: F) -> Self {
        let flags = flags.into();
        let mut builder = Self::new(sequence_type);
        builder.id(flags.test(Flag::Id));
        builder.quality(flags.test(Flag::Quality));
        builder.sequence(flags.test(Flag::Sequence));
        builder.comment(flags.test(Flag::Comment));
        builder
    }

    /// Whether or not to encode the identifier of each record.
    #[inline]
    pub fn id(&mut self, id: bool) -> &mut Self {
        self.id = id;
        self
    }

    /// Whether or not to encode the comment of each record.
    #[inline]
    pub fn comment(&mut self, comment: bool) -> &mut Self {
        self.comment = comment;
        self
    }

    /// Whether or not to encode the sequence of each record.
    #[inline]
    pub fn sequence(&mut self, sequence: bool) -> &mut Self {
        self.sequence = sequence;
        self
    }

    /// Whether or not to decode the quality of each record.
    #[inline]
    pub fn quality(&mut self, quality: bool) -> &mut Self {
        self.quality = quality;
        self
    }

    /// The compression level to use for `zstd` compression.
    ///
    /// Pass `0` to use the default `zstd` value, otherwise any
    /// integer in range 1-22. See [`zstd::stream::write::Encoder`]
    /// for more information.
    #[inline]
    pub fn compression_level(&mut self, level: i32) -> &mut Self {
        self.compression_level = level;
        self
    }

    /// Create a new compressed writer using a storage buffer.
    fn new_buffer<'z, S: Storage>(
        &self,
        storage: &S,
    ) -> Result<zstd::Encoder<'z, S::Buffer>, IoError> {
        let mut buffer = zstd::Encoder::new(storage.create_buffer()?, self.compression_level)?;
        buffer.include_magicbytes(false)?;
        Ok(buffer)
    }

    /// Consume the builder to get an encoder using in-memory storage.
    #[inline]
    pub fn with_memory<'z>(&self) -> Result<Encoder<'z, Memory>, IoError> {
        self.with_storage(Memory)
    }

    /// Consume the builder to get an encoder using the given storage.
    pub fn with_storage<'z, S: Storage>(&self, storage: S) -> Result<Encoder<'z, S>, IoError> {
        let mut header = Header::default();

        header.sequence_type = self.sequence_type;
        if self.sequence_type == SequenceType::Dna {
            header.format_version = FormatVersion::V1;
        } else {
            header.format_version = FormatVersion::V2;
        }

        if self.id {
            header.flags.set(Flag::Id);
        }
        if self.comment {
            header.flags.set(Flag::Comment);
        }
        if self.sequence {
            header.flags.set(Flag::Sequence);
            header.flags.set(Flag::Length);
        }
        if self.quality {
            header.flags.set(Flag::Quality);
            header.flags.set(Flag::Length);
        }

        let lens = self.new_buffer(&storage)?;
        let id = if self.id {
            Some(WriteCounter::new(self.new_buffer(&storage)?))
        } else {
            None
        };
        let com = if self.comment {
            Some(WriteCounter::new(self.new_buffer(&storage)?))
        } else {
            None
        };
        let seq = if self.sequence {
            Some(WriteCounter::new(SequenceWriter::new(
                self.sequence_type,
                self.new_buffer(&storage)?,
            )))
        } else {
            None
        };
        let qual = if self.quality {
            Some(WriteCounter::new(self.new_buffer(&storage)?))
        } else {
            None
        };

        Ok(Encoder {
            header,
            storage,
            seq,
            qual,
            com,
            id,
            len: WriteCounter::new(lens),
        })
    }
}

/// An encoder for Nucleotide Archive Format files.
///
/// NAF archives decomposes data into separate content blocks, which means
/// a NAF file can only be written to a file once all the records have been
/// processed. The encoder uses a temporary storage to store the
/// intermediate data, with an abstract interface declared as the
/// [`Storage`] trait.
pub struct Encoder<'z, S: Storage> {
    header: Header,
    storage: S,
    id: Option<WriteCounter<zstd::Encoder<'z, S::Buffer>>>,
    len: WriteCounter<zstd::Encoder<'z, S::Buffer>>,
    com: Option<WriteCounter<zstd::Encoder<'z, S::Buffer>>>,
    seq: Option<WriteCounter<SequenceWriter<zstd::Encoder<'z, S::Buffer>>>>,
    qual: Option<WriteCounter<zstd::Encoder<'z, S::Buffer>>>,
    // mask: WriteCounter<zstd::Encoder<'z, S::Buffer>>,
}

impl Encoder<'_, Memory> {
    /// Create a new encoder for the given sequence type using memory buffers.
    ///
    /// Use [`Encoder::from_storage`] to specificy a different storage type,
    /// such as a [`tempfile::TempDir`] to store the temporary compressed
    /// blocks into a temporary directory.
    pub fn new(sequence_type: SequenceType) -> Result<Self, IoError> {
        Self::from_storage(sequence_type, Default::default())
    }
}

impl<S: Storage> Encoder<'_, S> {
    pub fn from_storage(sequence_type: SequenceType, storage: S) -> Result<Self, IoError> {
        EncoderBuilder::new(sequence_type).with_storage(storage)
    }

    pub fn push(&mut self, record: &Record) -> Result<(), IoError> {
        if let Some(id_writer) = self.id.as_mut() {
            if let Some(id) = record.id.as_ref() {
                id_writer.write_all(id.as_bytes())?;
                id_writer.write_all(b"\0")?;
            } else {
                panic!("missing ids")
            }
        }

        if let Some(com_writer) = self.com.as_mut() {
            if let Some(com) = record.comment.as_ref() {
                com_writer.write_all(com.as_bytes())?;
                com_writer.write_all(b"\0")?;
                com_writer.flush()?;
            } else {
                panic!("missing comment");
            }
        }

        if let Some(seq_writer) = self.seq.as_mut() {
            if let Some(seq) = record.sequence.as_ref() {
                let length = seq.len();
                write_length(length as u64, &mut self.len)?;
                seq_writer.write(seq.as_bytes())?;
                seq_writer.flush()?;
            } else {
                panic!("missing sequence")
            }
        }

        if let Some(qual_writer) = self.qual.as_mut() {
            if let Some(qual) = record.quality.as_ref() {
                let length = qual.len();
                write_length(length as u64, &mut self.len)?;
                qual_writer.write_all(qual.as_bytes())?;
                qual_writer.flush()?;
            } else {
                panic!("missing quality")
            }
        }

        self.header.number_of_sequences += 1;
        Ok(())
    }

    pub fn write<W: Write>(self, mut file: W) -> Result<(), IoError> {
        // --- header ---
        file.write_all(&[0x01, 0xF9, 0xEC])?; // format descriptor

        if self.header.format_version == FormatVersion::V1 {
            file.write_all(&[
                self.header.format_version as u8,
                self.header.flags.into(),
                self.header.name_separator as u8,
            ])?;
        } else {
            file.write_all(&[
                self.header.format_version as u8,
                self.header.sequence_type as u8,
                self.header.flags.into(),
                self.header.name_separator as u8,
            ])?;
        }

        write_variable_length(self.header.line_length, &mut file)?;
        write_variable_length(self.header.number_of_sequences, &mut file)?;

        // -- ids ---

        if let Some(id_writer) = self.id {
            let og_ids = id_writer.len() as u64;
            let mut ids_buffer = id_writer.into_inner().finish()?;
            ids_buffer.flush()?;

            write_variable_length(og_ids, &mut file)?;
            write_variable_length(self.storage.buffer_length(&ids_buffer)? as u64, &mut file)?;
            self.storage.write_buffer(ids_buffer, &mut file)?;
        }

        // -- com ---

        if let Some(com_writer) = self.com {
            let og_com = com_writer.len() as u64;
            let mut com_buffer = com_writer.into_inner().finish()?;
            com_buffer.flush()?;

            write_variable_length(og_com, &mut file)?;
            write_variable_length(self.storage.buffer_length(&com_buffer)? as u64, &mut file)?;
            self.storage.write_buffer(com_buffer, &mut file)?;
        }

        // -- lengths --

        let og_lens = self.len.len() as u64;
        let mut len_buffer = self.len.into_inner().finish()?;
        len_buffer.flush()?;

        write_variable_length(og_lens, &mut file)?;
        write_variable_length(self.storage.buffer_length(&len_buffer)? as u64, &mut file)?;
        self.storage.write_buffer(len_buffer, &mut file)?;

        // -- seq --

        if let Some(seq_writer) = self.seq {
            let og_seqs = seq_writer.len() as u64;
            let mut seq_buffer = seq_writer.into_inner().into_inner()?.finish()?;
            seq_buffer.flush()?;

            write_variable_length(og_seqs, &mut file)?;
            write_variable_length(self.storage.buffer_length(&seq_buffer)? as u64, &mut file)?;
            self.storage.write_buffer(seq_buffer, &mut file)?;
        }

        // -- qual --

        if let Some(qual_writer) = self.qual {
            let og_qual = qual_writer.len() as u64;
            let mut qual_buffer = qual_writer.into_inner().finish()?;
            qual_buffer.flush()?;

            write_variable_length(og_qual, &mut file)?;
            write_variable_length(self.storage.buffer_length(&qual_buffer)? as u64, &mut file)?;
            self.storage.write_buffer(qual_buffer, &mut file)?;
        }

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
        let r1 = Record {
            id: Some("r1".into()),
            comment: Some("record 1".into()),
            sequence: Some("MYYK".into()),
            ..Default::default()
        };
        encoder.push(&r1).unwrap();

        let r2 = Record {
            id: Some("r2".into()),
            comment: Some("record 2".into()),
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
        let mut encoder = Encoder::from_storage(SequenceType::Dna, tempdir).unwrap();
        let r1 = Record {
            id: Some("r1".into()),
            comment: Some("record 1".into()),
            sequence: Some("ATTATTGC".into()),
            ..Default::default()
        };
        encoder.push(&r1).unwrap();

        let r2 = Record {
            id: Some("r2".into()),
            comment: Some("record 2".into()),
            sequence: Some("ATATGVBGD".into()),
            ..Default::default()
        };
        encoder.push(&r2).unwrap();

        let mut f = std::fs::File::create("/tmp/test2.naf").unwrap();
        encoder.write(&mut f).unwrap();
        f.flush().unwrap();

        let decoder = crate::Decoder::from_path("/tmp/test2.naf").unwrap();
        let records = decoder.collect::<Vec<_>>();
        assert_eq!(records.len(), 2);
    }
}
