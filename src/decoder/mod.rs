use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::iter::FusedIterator;
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

type ZstdDecoder<'z, R> =
    BufReader<zstd::stream::read::Decoder<'z, BufReader<IoSlice<BufReader<R>>>>>;

pub struct Decoder<'z, R: Read + Seek> {
    header: Header,
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
    pub fn new(r: R) -> Result<Self, Error> {
        let mut reader = BufReader::with_capacity(4096, r);

        let buffer = reader.fill_buf()?;
        let header = match self::parser::header(buffer) {
            Ok((i, header)) => {
                let consumed = buffer.len() - i.len();
                reader.consume(consumed);
                header
            }
            Err(nom::Err::Incomplete(_)) => unreachable!(),
            Err(nom::Err::Error(e) | nom::Err::Failure(e)) => {
                return Err(Error::from(e));
            }
        };

        if header.flags().has_title() {
            let buf = reader.buffer();
            let (i, _title) = self::parser::title(buf)?;
            let consumed = buf.len() - i.len();
            reader.consume(consumed);
        }

        let reader_rc = Rc::new(RwLock::new(reader));
        macro_rules! setup_block {
            ($reader_rc:ident, $block:ident) => {
                let _length: u64;
                setup_block!($reader_rc, $block, _length);
            };
            ($reader_rc:ident, $block:ident, $block_length:ident) => {
                // create a local copy of the reader that we can access
                let tee = $reader_rc.clone();
                let mut handle = $reader_rc.write().unwrap();
                // decode the block size
                let buf = handle.fill_buf()?;
                let (i, original_size) = self::parser::variable_u64(buf)?;
                let (i, compressed_size) = self::parser::variable_u64(i)?;
                $block_length = original_size;
                let consumed = buf.len() - i.len();
                handle.consume(consumed);
                // setup the independent decoder for the block
                let pos = handle.stream_position()?;
                let tee_slice = IoSlice::new(tee, pos, pos + compressed_size);
                let mut decoder = zstd::stream::read::Decoder::new(tee_slice)?;
                decoder.include_magicbytes(false)?;
                $block = Some(BufReader::new(decoder));
                // skip the block with the main reader
                handle.seek(SeekFrom::Current(compressed_size as i64))?;
            };
        }

        let ids_block;
        if header.flags().has_ids() {
            setup_block!(reader_rc, ids_block);
        } else {
            ids_block = None;
        }

        let comments_block;
        if header.flags().has_comments() {
            setup_block!(reader_rc, comments_block);
        } else {
            comments_block = None;
        }

        let lengths_block;
        if header.flags().has_lengths() {
            setup_block!(reader_rc, lengths_block);
        } else {
            lengths_block = None;
        }

        let mask_block;
        if header.flags().has_mask() {
            setup_block!(reader_rc, mask_block);
        } else {
            mask_block = None;
        }

        let sequence_block;
        let mut sequence_length = 0;
        if header.flags().has_sequence() {
            setup_block!(reader_rc, sequence_block, sequence_length);
        } else {
            sequence_block = None;
        }

        let quality_block;
        if header.flags().has_quality() {
            setup_block!(reader_rc, quality_block);
        } else {
            quality_block = None;
        }

        Ok(Self {
            ids: ids_block.map(CStringReader::new),
            com: comments_block.map(CStringReader::new),
            len: lengths_block.map(LengthReader::new),
            seq: sequence_block.map(|x| SequenceReader::new(x, header.sequence_type())),
            qual: quality_block.map(|x| SequenceReader::new(x, SequenceType::Text)),
            mask: mask_block.map(|x| MaskReader::new(x, sequence_length)),

            n: 0,

            // reader,
            header,
            unit: MaskUnit::Unmasked(0),
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

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

    fn mask_sequence(&mut self, sequence: &mut String) -> Result<(), Error> {
        let mut mask = self.unit.clone();
        let mut seq = sequence.as_mut_str();

        if let Some(mut mask_reader) = self.mask.as_mut() {
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
                mask = match self.mask.as_mut().unwrap().next() {
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
}
