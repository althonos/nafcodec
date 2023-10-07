use std::fmt::Debug;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::iter::FusedIterator;
use std::path::Path;
use std::rc::Rc;
use std::sync::RwLock;

mod ioslice;
mod parser;
mod reader;

use self::ioslice::IoSlice;
use self::reader::*;
use crate::data::Header;
use crate::data::MaskUnit;
use crate::data::Record;
use crate::data::SequenceType;
use crate::error::Error;

/// The wrapper used to decode Zstandard stream.
type ZstdDecoder<'z, R> =
    BufReader<zstd::stream::read::Decoder<'z, BufReader<IoSlice<BufReader<R>>>>>;

/// A builder to configure and initialize a decoder.
#[derive(Debug, Clone)]
pub struct DecoderBuilder {
    buffer_size: usize,
    quality: bool,
    sequence: bool,
    mask: bool,
}

impl DecoderBuilder {
    pub fn new() -> Self {
        Self {
            buffer_size: 4096,
            quality: true,
            sequence: true,
            mask: true,
        }
    }

    /// Build a decoder with this configuration that reads data from the given file path.
    pub fn from_path<'z, P: AsRef<Path>>(&self, path: P) -> Result<Decoder<'z, File>, Error> {
        File::open(path.as_ref())
            .map_err(Error::from)
            .and_then(|f| self.from_reader(f))
    }

    /// Build a decoder with this configuration that reads data from `reader`.
    pub fn from_reader<'z, R: Read + Seek>(&self, reader: R) -> Result<Decoder<'z, R>, Error> {
        let mut r = BufReader::with_capacity(self.buffer_size, reader);

        let buffer = r.fill_buf()?;
        let header = match self::parser::header(buffer) {
            Ok((i, header)) => {
                let consumed = buffer.len() - i.len();
                r.consume(consumed);
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

        if header.flags().has_title() {
            let buf = r.buffer();
            let (i, _title) = self::parser::title(buf)?;
            let consumed = buf.len() - i.len();
            r.consume(consumed);
        }

        let rc = Rc::new(RwLock::new(r));
        macro_rules! setup_block {
            ($flag:expr, $use_block:expr, $rc:ident, $block:ident) => {
                let _length: u64;
                setup_block!($flag, $use_block, $rc, $block, _length);
            };
            ($flag:expr, $use_block:expr, $rc:ident, $block:ident, $block_length:ident) => {
                let $block;
                if $flag {
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
        setup_block!(flags.has_ids(), true, rc, ids_block);
        setup_block!(flags.has_comments(), true, rc, com_block);
        setup_block!(flags.has_lengths(), true, rc, len_block);
        setup_block!(flags.has_mask(), self.mask, rc, mask_block);
        setup_block!(flags.has_sequence(), self.sequence, rc, seq_block, seqlen);
        setup_block!(flags.has_quality(), self.quality, rc, quality_block);

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

    /// The buffer size to use while reading.
    ///
    /// Note that `Decoder` uses a lot of buffered I/O, and that more than
    /// one buffer will be created. Nevertheless, a higher value will reduce
    /// the necessity to [`seek`] the reader while reading the different
    /// blocks.
    pub fn buffer_size(&mut self, buffer_size: usize) -> &mut Self {
        self.buffer_size = buffer_size;
        self
    }

    /// Whether or not to decode the sequence string if available.
    pub fn sequence(&mut self, sequence: bool) -> &mut Self {
        self.sequence = sequence;
        self
    }

    /// Whether or not to decode the quality string if available.
    pub fn quality(&mut self, quality: bool) -> &mut Self {
        self.quality = quality;
        self
    }

    /// Whether or not to perform region masking in the output sequence.
    pub fn mask(&mut self, mask: bool) -> &mut Self {
        self.mask = mask;
        self
    }
}

/// A decoder for Nucleotide Archive Format files.
///
/// The internal reader is shared and accessed non-sequentially to read the
/// different block components of the archive. This means that the internal
/// file heavily make use of [`seek`], so make sure that the actual object
/// has a fast seeking implementation.
pub struct Decoder<'z, R: Read + Seek> {
    header: Header,

    reader: Rc<RwLock<BufReader<R>>>,

    ids: Option<CStringReader<ZstdDecoder<'z, R>>>,
    com: Option<CStringReader<ZstdDecoder<'z, R>>>,
    len: Option<LengthReader<ZstdDecoder<'z, R>>>,
    seq: Option<SequenceReader<ZstdDecoder<'z, R>>>,
    qual: Option<SequenceReader<ZstdDecoder<'z, R>>>,
    mask: Option<MaskReader<ZstdDecoder<'z, R>>>,

    n: usize,
    unit: MaskUnit,
}

impl<R: Read + Seek> Decoder<'_, R> {
    /// Create a new decoder from the given reader.
    ///
    /// This constructor is a shortcut for `DecoderBuilder::new().from_reader(reader)`.
    /// Use `DecoderBuilder` to configure a decoder with more options.
    pub fn new(r: R) -> Result<Self, Error> {
        DecoderBuilder::new().from_reader(r)
    }

    /// Get the header extracted from the archive.
    ///
    /// The NAF header contains useful metadata which are decoded before
    /// starting to decode the rest of the archive, such as the total number
    /// of sequences (useful for building a progress bar) or the line length
    /// (useful for writing the decoded sequences in FASTA format).
    pub fn header(&self) -> &Header {
        &self.header
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
            .into_inner()
    }

    /// Attempt to read the next record from the archive.
    ///
    /// This function expects that a record is available; use `Decoder::next`
    /// to check beforehand whether all sequences were read from the archive.
    fn next_record(&mut self) -> Result<Record, Error> {
        let id = self
            .ids
            .as_mut()
            .map(|r| r.next())
            .transpose()?
            .map(|id| id.into_string().expect("TODO"));
        let comment = self
            .com
            .as_mut()
            .map(|r| r.next())
            .transpose()?
            .map(|com| com.into_string().expect("TODO"));
        let length = self.len.as_mut().map(|r| r.next()).transpose()?;

        let mut sequence = None;
        let mut quality = None;
        if let Some(l) = length {
            sequence = self.seq.as_mut().map(|r| r.next(l)).transpose()?;
            quality = self.qual.as_mut().map(|r| r.next(l)).transpose()?;
            if let Some(mut seq) = sequence.as_mut() {
                self.mask_sequence(&mut seq)?;
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
    fn mask_sequence(&mut self, sequence: &mut String) -> Result<(), Error> {
        let mut mask = self.unit.clone();
        let mut seq = sequence.as_mut_str();

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

impl<R: Read + Seek> Iterator for Decoder<'_, R> {
    type Item = Result<Record, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.n as u64 >= self.header.number_of_sequences() {
            return None;
        }
        Some(self.next_record())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.header.number_of_sequences() as usize - self.n;
        (0, Some(remaining))
    }
}

impl<R: Read + Seek> FusedIterator for Decoder<'_, R> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::MaskUnit;

    const ARCHIVE: &[u8] = include_bytes!("../../data/LuxC.naf");

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
        const ARCHIVE: &[u8] = include_bytes!("../../data/masked.naf");
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
            .from_reader(std::io::Cursor::new(ARCHIVE))
            .unwrap();
        for record in decoder.map(Result::unwrap) {
            assert!(record.sequence.is_none());
        }
    }
}
