use std::io::Error as IoError;
use std::io::Write;

mod counter;
mod storage;
mod writer;

pub use self::storage::Memory;
pub use self::storage::Storage;

use self::counter::WriteCounter;
use self::writer::SequenceWriter;
use crate::error::Error;

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
    while l >= (u32::MAX as u64) {
        w.write_all(&u32::MAX.to_le_bytes()[..])?;
        l -= u32::MAX as u64;
    }
    let n = l as u32;
    w.write_all(&n.to_le_bytes()[..])
}

/// A builder to configure and initialize an [`Encoder`].
///
/// The fields to encode are *opt-in*: only the fields enabled through the
/// builder will be extracted from the [`Record`] passed to [`Encoder::push`]
/// and written to the archive.
///
/// For instance, to write a nucleotide archive containing only the sequence
/// and identifier of each record:
/// ```rust
/// # use nafcodec::*;
/// let encoder = EncoderBuilder::new(SequenceType::Dna)
///     .id(true)
///     .sequence(true)
///     .with_memory()
///     .unwrap();
/// ```
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
            id: false,
            quality: false,
            comment: false,
            sequence: false,
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
    pub fn with_memory<'z>(&self) -> Result<Encoder<'z, Memory>, Error> {
        self.with_storage(Memory)
    }

    /// Consume the builder to get an encoder using the given storage.
    pub fn with_storage<'z, S: Storage>(&self, storage: S) -> Result<Encoder<'z, S>, Error> {
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

impl<S: Storage> Encoder<'_, S> {
    /// Push a [`Record`] to the archive.
    ///
    /// The records contents are written to the temporary storage used
    /// internally by the [`Encoder`], but the [`Encoder::write`] method
    /// needs to be called once all records have been added to build the
    /// final archive.
    pub fn push(&mut self, record: &Record) -> Result<(), Error> {
        let mut written_length = None;

        if let Some(&length) = record.length.as_ref() {
            write_length(length as u64, &mut self.len)?;
            written_length = Some(length as u64);
        }

        if let Some(id_writer) = self.id.as_mut() {
            if let Some(id) = record.id.as_ref() {
                id_writer.write_all(id.as_bytes())?;
                id_writer.write_all(b"\0")?;
            } else {
                return Err(Error::MissingField("id"));
            }
        }

        if let Some(com_writer) = self.com.as_mut() {
            if let Some(com) = record.comment.as_ref() {
                com_writer.write_all(com.as_bytes())?;
                com_writer.write_all(b"\0")?;
                com_writer.flush()?;
            } else {
                return Err(Error::MissingField("comment"));
            }
        }

        if let Some(seq_writer) = self.seq.as_mut() {
            if let Some(seq) = record.sequence.as_ref() {
                match written_length {
                    Some(length) => {
                        if length != seq.len() as u64 {
                            return Err(Error::InvalidLength);
                        }
                    }
                    None => {
                        let length = seq.len();
                        write_length(length as u64, &mut self.len)?;
                        written_length = Some(length as u64);
                    }
                }
                if let Err(e) = seq_writer.write(seq.as_bytes()) {
                    if e.kind() == std::io::ErrorKind::InvalidData {
                        return Err(Error::InvalidSequence);
                    } else {
                        return Err(Error::Io(e));
                    }
                }
                seq_writer.flush()?;
            } else {
                return Err(Error::MissingField("sequence"));
            }
        }

        if let Some(qual_writer) = self.qual.as_mut() {
            if let Some(qual) = record.quality.as_ref() {
                match written_length {
                    Some(length) => {
                        if length != qual.len() as u64 {
                            return Err(Error::InvalidLength);
                        }
                    }
                    None => {
                        let length = qual.len();
                        write_length(length as u64, &mut self.len)?;
                        // written_length = Some(length as u64);
                    }
                }
                qual_writer.write_all(qual.as_bytes())?;
                qual_writer.flush()?;
            } else {
                return Err(Error::MissingField("quality"));
            }
        }

        self.header.number_of_sequences += 1;
        Ok(())
    }

    /// Finalize the archive and write it to the given writer.
    ///
    /// This method consumes the [`Encoder`], since it cannot receive any
    /// additional [`Record`] after the compressed blocks have been
    /// finalized.
    pub fn write<W: Write>(self, mut file: W) -> Result<(), Error> {
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

        macro_rules! write_block {
            ($field:expr) => {
                write_block!($field, |x| Result::<_, Error>::Ok(x))
            };
            ($field:expr, $getbuffer:expr) => {
                if let Some(writer) = $field {
                    let uncompressed_length = writer.len() as u64;
                    let mut buffer = $getbuffer(writer.into_inner())?.finish()?;
                    buffer.flush()?;
                    let compressed_length = self.storage.buffer_length(&buffer)?;

                    write_variable_length(uncompressed_length, &mut file)?;
                    write_variable_length(compressed_length as u64, &mut file)?;
                    self.storage.write_buffer(buffer, &mut file)?;
                }
            };
        }

        write_block!(self.id);
        write_block!(self.com);
        write_block!(Some(self.len));
        write_block!(self.seq, |f: SequenceWriter<_>| f.into_inner());
        write_block!(self.qual);

        file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_length() {
        let mut v = Vec::<u8>::new();

        macro_rules! assert_encoded {
            ($n:expr, $encoding:expr) => {
                v.clear();
                write_variable_length($n, &mut v).unwrap();
                assert_eq!(v, &$encoding);
            };
        }

        assert_encoded!(0, [0x00]);
        assert_encoded!(1, [0x01]);
        assert_encoded!(9, [0x09]);
        assert_encoded!(10, [0x0a]);
        assert_encoded!(100, [0x64]);
        assert_encoded!(127, [0x7f]);
        assert_encoded!(128, [0x81, 0x00]);
        assert_encoded!(129, [0x81, 0x01]);
        assert_encoded!(34359738367, [0xff, 0xff, 0xff, 0xff, 0x7f]);
        assert_encoded!(34359738368, [0x81, 0x80, 0x80, 0x80, 0x80, 0x00]);
    }

    #[test]
    fn encoder_memory() {
        let mut encoder = EncoderBuilder::new(SequenceType::Protein)
            .id(true)
            .sequence(true)
            .with_memory()
            .unwrap();
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
        let mut encoder = EncoderBuilder::new(SequenceType::Dna)
            .id(true)
            .sequence(true)
            .with_storage(tempdir)
            .unwrap();
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
