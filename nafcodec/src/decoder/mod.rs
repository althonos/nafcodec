use std::borrow::Cow;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Seek;
use std::io::SeekFrom;
use std::iter::FusedIterator;
use std::path::Path;
use std::sync::RwLock;

mod ioslice;
mod parser;
mod reader;

use self::ioslice::IoSlice;
use self::reader::CStringReader;
use self::reader::LengthReader;
use self::reader::MaskReader;
use self::reader::SequenceReader;
use super::Rc;
use crate::data::Flag;
use crate::data::Flags;
use crate::data::Header;
use crate::data::MaskUnit;
use crate::data::Record;
use crate::data::SequenceType;
use crate::error::Error;

/// The wrapper used to decode Zstandard stream.
type ZstdDecoder<'z, R> = BufReader<zstd::Decoder<'z, BufReader<IoSlice<R>>>>;

/// A builder to configure and initialize a [`Decoder`].
///
/// The fields to decode are *opt-out*: by default, the [`Decoder`] will
/// extract all available fields found in the archive into each [`Record`].
/// Use the provided methods to avoid decoding uneeded fields.
///
/// For instance, to read a nucleotide archive and only extract
/// sequences names, skipping decoding of sequences:
/// ```rust
/// let decoder = nafcodec::DecoderBuilder::new()
///     .sequence(false)
///     .quality(false)
///     .with_path("../data/phix.naf")
///     .unwrap();
/// for record in decoder.map(Result::unwrap) {
///     println!(">{}", record.id.unwrap());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct DecoderBuilder {
    buffer_size: usize,
    id: bool,
    comment: bool,
    sequence: bool,
    quality: bool,
    mask: bool,
}

impl DecoderBuilder {
    /// Create a new decoder builder with default parameters.
    ///
    /// By default, all fields are extracted if they are available in
    /// the header.
    pub fn new() -> Self {
        Self {
            buffer_size: 4096,
            id: true,
            comment: true,
            sequence: true,
            quality: true,
            mask: true,
        }
    }

    /// Create a new decoder builder from the given flags.
    ///
    /// This constructor can be used as a shortcut to setup decoding
    /// of a subset of supported fields. For instance, to read only the
    /// sequence identifiers and quality lines from an archive:
    /// ```
    /// # use nafcodec::{DecoderBuilder, Flag};
    /// let mut decoder = DecoderBuilder::from_flags(Flag::Id | Flag::Quality)
    ///     .with_path("../data/phix.naf")
    ///     .unwrap();
    ///
    /// let record = decoder.next().unwrap().unwrap();
    /// assert!(record.sequence.is_none());
    /// assert!(record.quality.is_some());
    /// ```
    pub fn from_flags<F: Into<Flags>>(flags: F) -> Self {
        let flags = flags.into();
        let mut builder = Self::new();
        builder.quality(flags.test(Flag::Quality));
        builder.sequence(flags.test(Flag::Sequence));
        builder.mask(flags.test(Flag::Mask));
        builder.comment(flags.test(Flag::Comment));
        builder
    }

    /// The buffer size to use while reading.
    ///
    /// Note that [`Decoder`] uses a lot of buffered I/O, and that more than
    /// one buffer will be created. Nevertheless, a higher value will reduce
    /// the necessity to seek the reader while reading the different blocks.
    ///
    /// By default, a buffer size of 4KiB is used for each internal buffer.
    pub fn buffer_size(&mut self, buffer_size: usize) -> &mut Self {
        self.buffer_size = buffer_size;
        self
    }

    /// Whether or not to decode the sequence identifiers if available.
    #[inline]
    pub fn id(&mut self, id: bool) -> &mut Self {
        self.id = id;
        self
    }

    /// Whether or not to decode the sequence comment if available.
    #[inline]
    pub fn comment(&mut self, comment: bool) -> &mut Self {
        self.comment = comment;
        self
    }

    /// Whether or not to decode the sequence string if available.
    #[inline]
    pub fn sequence(&mut self, sequence: bool) -> &mut Self {
        self.sequence = sequence;
        self
    }

    /// Whether or not to decode the quality string if available.
    #[inline]
    pub fn quality(&mut self, quality: bool) -> &mut Self {
        self.quality = quality;
        self
    }

    /// Whether or not to perform region masking in the output sequence.
    #[inline]
    pub fn mask(&mut self, mask: bool) -> &mut Self {
        self.mask = mask;
        self
    }

    /// Consume the builder to get a decoder reading data from the given buffer.
    pub fn with_bytes<'data, 'z>(
        &self,
        bytes: &'data [u8],
    ) -> Result<Decoder<'z, BufReader<Cursor<&'data [u8]>>>, Error> {
        self.with_reader(BufReader::new(Cursor::new(bytes)))
    }

    /// Consume the builder to get a decoder reading a file at the given path.
    pub fn with_path<'z, P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Decoder<'z, BufReader<File>>, Error> {
        File::open(path.as_ref())
            .map_err(Error::from)
            .and_then(|f| self.with_reader(std::io::BufReader::new(f)))
    }

    /// Consume the builder to get a decoder reading data from `reader`.
    pub fn with_reader<'z, R: BufRead + Seek>(
        &self,
        mut reader: R,
    ) -> Result<Decoder<'z, R>, Error> {
        let buffer = reader.fill_buf()?;
        let header = match self::parser::header(buffer) {
            Ok((i, header)) => {
                let consumed = buffer.len() - i.len();
                reader.consume(consumed);
                header
            }
            Err(nom::Err::Incomplete(_)) => {
                return Err(Error::from(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "failed to read header",
                )));
            }
            Err(nom::Err::Error(e) | nom::Err::Failure(e)) => {
                return Err(Error::from(e));
            }
        };

        if header.flags().test(Flag::Title) {
            let buf = reader.fill_buf()?;
            let (i, _title) = self::parser::title(buf)?;
            let consumed = buf.len() - i.len();
            reader.consume(consumed);
        }

        let rc = Rc::new(RwLock::new(reader));
        macro_rules! setup_block {
            ($flags:expr, $flag:ident, $use_block:expr, $rc:ident, $block:ident) => {
                let _length: u64;
                setup_block!($flags, $flag, $use_block, $rc, $block, _length);
            };
            ($flags:expr, $flag:ident, $use_block:expr, $rc:ident, $block:ident, $block_length:ident) => {
                let $block;
                if $flags.test(Flag::$flag) {
                    // create a local copy of the reader that we can access
                    let tee = $rc.clone();
                    let mut handle = $rc.write().unwrap();
                    // decode the block size
                    let buf = handle.fill_buf()?;
                    let (i, original_size) = self::parser::variable_u64(buf)?;
                    let (i, compressed_size) = self::parser::variable_u64(i)?;
                    $block_length = original_size;
                    let consumed = buf.len() - i.len();
                    handle.consume(consumed);
                    // setup the independent decoder for the block
                    if $use_block {
                        let pos = handle.stream_position()?;
                        let tee_slice = IoSlice::new(tee, pos, pos + compressed_size);
                        let mut decoder = zstd::stream::read::Decoder::new(tee_slice)?;
                        decoder.include_magicbytes(false)?;
                        $block = Some(BufReader::with_capacity(self.buffer_size, decoder));
                    } else {
                        $block = None;
                    }
                    // skip the block with the main reader
                    handle.seek(SeekFrom::Current(compressed_size as i64))?;
                } else {
                    $block = None;
                }
            };
        }

        let flags = header.flags();
        let mut seqlen = 0;
        setup_block!(flags, Id, self.id, rc, ids_block);
        setup_block!(flags, Comment, self.comment, rc, com_block);
        setup_block!(flags, Length, true, rc, len_block);
        setup_block!(flags, Mask, self.mask, rc, mask_block);
        setup_block!(flags, Sequence, self.sequence, rc, seq_block, seqlen);
        setup_block!(flags, Quality, self.quality, rc, quality_block);

        Ok(Decoder {
            ids: ids_block.map(CStringReader::new),
            com: com_block.map(CStringReader::new),
            len: len_block.map(LengthReader::new),
            seq: seq_block.map(|x| SequenceReader::new(x, header.sequence_type())),
            qual: quality_block.map(|x| SequenceReader::new(x, SequenceType::Text)),
            mask: mask_block.map(|x| MaskReader::new(x, seqlen)),
            n: 0,
            header,
            reader: rc,
            unit: MaskUnit::Unmasked(0),
        })
    }
}

impl Default for DecoderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A decoder for Nucleotide Archive Format files.
///
/// The internal reader is shared and accessed non-sequentially to read the
/// different block components of the archive. This means that the internal
/// file heavily make use of [`Seek::seek`], so make sure that the actual
/// type has a fast seeking implementation.
///
/// By default, the decoder will decode all available fields, which may not
/// be needed. Use a [`DecoderBuilder`] to configure decoding of individual
/// fields.
///
/// # Thread safety
///
/// By default, `Decoder` objects are not [`Send`] because they use reference
/// counting to share the reader between the different block parsers. Compile
/// the crate with the *arc* feature to use [`Arc`] instead of [`Rc`], making
/// the decoder [`Send`].
///
/// [`Rc`]: https://doc.rust-lang.org/nightly/std/rc/struct.Rc.html
/// [`Arc`]: https://doc.rust-lang.org/nightly/std/sync/struct.Arc.html
pub struct Decoder<'z, R: BufRead + Seek> {
    header: Header,
    reader: Rc<RwLock<R>>,
    ids: Option<CStringReader<ZstdDecoder<'z, R>>>,
    com: Option<CStringReader<ZstdDecoder<'z, R>>>,
    len: Option<LengthReader<ZstdDecoder<'z, R>>>,
    seq: Option<SequenceReader<ZstdDecoder<'z, R>>>,
    qual: Option<SequenceReader<ZstdDecoder<'z, R>>>,
    mask: Option<MaskReader<ZstdDecoder<'z, R>>>,
    n: usize,
    unit: MaskUnit,
}

impl Decoder<'_, BufReader<File>> {
    /// Create a new decoder from the given path.
    ///
    /// This constructor is a shortcut for `DecoderBuilder::new().from_path(path)`.
    /// Use [`DecoderBuilder`](./struct.DecoderBuilder.html) to configure a decoder
    /// with more options.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        DecoderBuilder::new().with_path(path)
    }
}

impl<R: BufRead + Seek> Decoder<'_, R> {
    /// Create a new decoder from the given reader.
    ///
    /// This constructor is a shortcut for `DecoderBuilder::new().with_reader(reader)`.
    /// Use [`DecoderBuilder`](./struct.DecoderBuilder.html) to configure a
    /// decoder with more options.
    pub fn new(reader: R) -> Result<Self, Error> {
        DecoderBuilder::new().with_reader(reader)
    }

    /// Get the header extracted from the archive.
    ///
    /// The NAF header contains useful metadata which are decoded before
    /// starting to decode the rest of the archive, such as the total number
    /// of sequences (useful for building a progress bar) or the line length
    /// (useful for writing the decoded sequences in FASTA format).
    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Get the type of sequence in the archive being decoded.
    ///
    /// This method is a shortcut for `self.header().sequence_type()`.
    #[inline]
    pub fn sequence_type(&self) -> SequenceType {
        self.header().sequence_type()
    }

    /// Extract the internal reader.
    ///
    /// Note that the internal reader may have been advanced even if no
    /// records were obtained from the decoder yet, since at least the header
    /// needs to be decoded to obtain a working decoder.
    pub fn into_inner(self) -> R {
        let reader = self.reader.clone();
        drop(self);
        Rc::into_inner(reader)
            .expect("reference count should be 1 after decoder is dropped")
            .into_inner()
            .expect("lock shouldn't be poisoned")
    }

    /// Attempt to read the next record from the archive.
    ///
    /// This function expects that a record is available; use `Decoder::next`
    /// to check beforehand whether all sequences were read from the archive.
    fn next_record(&mut self) -> Result<Record<'static>, Error> {
        let id = self
            .ids
            .as_mut()
            .and_then(|r| r.next())
            .transpose()?
            .map(|id| id.into_string().map(Cow::Owned).expect("TODO"));
        let comment = self
            .com
            .as_mut()
            .and_then(|r| r.next())
            .transpose()?
            .map(|com| com.into_string().map(Cow::Owned).expect("TODO"));
        let length = self.len.as_mut().and_then(|r| r.next()).transpose()?;

        let mut sequence: Option<Cow<'static, str>> = None;
        let mut quality = None;
        if let Some(l) = length {
            sequence = self
                .seq
                .as_mut()
                .map(|r| r.next(l))
                .transpose()?
                .map(Cow::Owned);
            quality = self
                .qual
                .as_mut()
                .map(|r| r.next(l))
                .transpose()?
                .map(Cow::Owned);
            if let Some(seq) = sequence.as_mut() {
                self.mask_sequence(seq.to_mut())?;
            }
        }

        self.n += 1;
        Ok(Record {
            id,
            comment,
            sequence,
            quality,
            length,
        })
    }

    /// Attempt to mask some regions of the given sequence.
    fn mask_sequence(&mut self, sequence: &mut str) -> Result<(), Error> {
        let mut mask = self.unit.clone();
        let mut seq = sequence;

        if let Some(mask_reader) = self.mask.as_mut() {
            loop {
                match mask {
                    MaskUnit::Masked(n) => {
                        if n < seq.len() as u64 {
                            seq[..n as usize].make_ascii_lowercase();
                            seq = &mut seq[n as usize..];
                        } else {
                            self.unit = MaskUnit::Masked(n - seq.len() as u64);
                            break;
                        }
                    }
                    MaskUnit::Unmasked(n) => {
                        if n < seq.len() as u64 {
                            seq = &mut seq[n as usize..];
                        } else {
                            self.unit = MaskUnit::Unmasked(n - seq.len() as u64);
                            break;
                        }
                    }
                }
                mask = match mask_reader.next() {
                    Some(Ok(x)) => x,
                    Some(Err(e)) => return Err(Error::Io(e)),
                    None => {
                        return Err(Error::Io(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "failed to get mask unit",
                        )))
                    }
                };
            }
        }

        Ok(())
    }
}

impl<R: BufRead + Seek> Iterator for Decoder<'_, R> {
    type Item = Result<Record<'static>, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.n as u64 >= self.header.number_of_sequences() {
            return None;
        }
        Some(self.next_record())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.header.number_of_sequences() as usize - self.n;
        (remaining, Some(remaining))
    }
}

impl<R: BufRead + Seek> ExactSizeIterator for Decoder<'_, R> {}

impl<R: BufRead + Seek> FusedIterator for Decoder<'_, R> {}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCHIVE: &[u8] = include_bytes!("../../../data/LuxC.naf");

    #[test]
    fn error_empty() {
        match Decoder::new(std::io::Cursor::new(b"")) {
            Ok(_decoder) => panic!("unexpected success"),
            Err(Error::Io(e)) => assert!(matches!(e.kind(), std::io::ErrorKind::UnexpectedEof)),
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn decoder() {
        let decoder = Decoder::new(std::io::Cursor::new(ARCHIVE)).unwrap();
        let records = decoder.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(records.len(), 12);
    }

    #[test]
    fn masks() {
        const ARCHIVE: &[u8] = include_bytes!("../../../data/masked.naf");
        let decoder = Decoder::new(std::io::Cursor::new(ARCHIVE)).unwrap();
        let mut mask_reader = decoder.mask.unwrap();
        assert_eq!(
            mask_reader.next().unwrap().unwrap(),
            MaskUnit::Unmasked(657)
        );
        assert_eq!(mask_reader.next().unwrap().unwrap(), MaskUnit::Masked(19));
        assert_eq!(
            mask_reader.next().unwrap().unwrap(),
            MaskUnit::Unmasked(635)
        );
        assert_eq!(mask_reader.next().unwrap().unwrap(), MaskUnit::Masked(39));
        assert_eq!(
            mask_reader.next().unwrap().unwrap(),
            MaskUnit::Unmasked(725)
        );
    }

    #[test]
    fn skip_sequence() {
        let decoder = DecoderBuilder::new()
            .sequence(false)
            .with_reader(std::io::Cursor::new(ARCHIVE))
            .unwrap();
        for record in decoder.map(Result::unwrap) {
            assert!(record.sequence.is_none());
        }
    }
}
