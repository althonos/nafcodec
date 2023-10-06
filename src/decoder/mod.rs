use std::ffi::CString;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use nom::IResult;
use slice::IoSlice;

pub mod reader;
pub mod parser;
pub mod tee;

use super::data::*;
use super::error::Error;
use self::reader::*;
use self::tee::Tee;


type ZstdDecoder<'z, R> =
    BufReader<zstd::stream::read::Decoder<'z, BufReader<IoSlice<BufReader<R>>>>>;

pub struct Decoder<'z, R: Read + Seek + Tee> {
    pub header: Header,
    reader: BufReader<R>,

    ids: Option<CStringReader<ZstdDecoder<'z, R>>>,
    com: Option<CStringReader<ZstdDecoder<'z, R>>>,
    len: Option<LengthReader<ZstdDecoder<'z, R>>>,
    seq: Option<SequenceReader<ZstdDecoder<'z, R>>>,
    qual: Option<TextReader<ZstdDecoder<'z, R>>>,

    n: usize,
}

impl<R: Read + Seek + Tee> Decoder<'_, R> {
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
            let (i, title) = self::parser::title(buf)?;
            let consumed = buf.len() - i.len();
            reader.consume(consumed);
        }

        macro_rules! setup_block {
            ($reader:ident, $block:ident) => {
                // decode size of the block
                let buf = $reader.fill_buf()?;
                let (i, original_size) = self::parser::variable_u64(buf)?;
                let (i, compressed_size) = self::parser::variable_u64(i)?;
                let consumed = buf.len() - i.len();
                $reader.consume(consumed);
                // setup the independent decoder for the block
                let pos = $reader.stream_position()?;
                let mut tee_reader = $reader.tee()?;
                let mut tee_slice = IoSlice::new(tee_reader, pos, pos + compressed_size)?;
                let mut decoder = zstd::stream::read::Decoder::new(tee_slice)?;
                decoder.include_magicbytes(false)?;
                $block = Some(BufReader::new(decoder));
                // skip the block with the main reader
                $reader.seek(SeekFrom::Current(compressed_size as i64))?;
            };
        }

        let ids_block;
        if header.flags().has_ids() {
            setup_block!(reader, ids_block);
        } else {
            ids_block = None;
        }

        let comments_block;
        if header.flags().has_comments() {
            setup_block!(reader, comments_block);
        } else {
            comments_block = None;
        }

        let lengths_block;
        if header.flags().has_lengths() {
            setup_block!(reader, lengths_block);
        } else {
            lengths_block = None;
        }

        let mask_block;
        if header.flags().has_mask() {
            setup_block!(reader, mask_block);
        } else {
            mask_block = None;
        }

        let sequence_block;
        if header.flags().has_sequence() {
            setup_block!(reader, sequence_block);
        } else {
            sequence_block = None;
        }

        let quality_block;
        if header.flags().has_quality() {
            setup_block!(reader, quality_block);
        } else {
            quality_block = None;
        }

        Ok(Self {
            ids: ids_block.map(CStringReader::new),
            com: comments_block.map(CStringReader::new),
            len: lengths_block.map(LengthReader::new),
            seq: sequence_block.map(|x| SequenceReader::new(x, header.sequence_type())),
            qual: quality_block.map(TextReader::new),

            n: 0,

            reader,
            header,
        })
    }

    pub fn next(&mut self) -> Option<Result<Record, Error>> {
        if self.n as u64 >= self.header.number_of_sequences() {
            return None;
        }

        let id = match self
            .ids
            .as_mut()
            .map(|reader| reader.next().map_err(Error::from))
        {
            None => None,
            Some(Ok(id)) => Some(id.into_string().expect("TODO")),
            Some(Err(e)) => return Some(Err(e)),
        };

        let comment = match self
            .com
            .as_mut()
            .map(|reader| reader.next().map_err(Error::from))
        {
            None => None,
            Some(Ok(com)) => Some(com.into_string().expect("TODO")),
            Some(Err(e)) => return Some(Err(e)),
        };

        let length = match self
            .len
            .as_mut()
            .map(|reader| reader.next().map_err(Error::from))
        {
            None => None,
            Some(Ok(len)) => Some(len),
            Some(Err(e)) => return Some(Err(e)),
        };

        let sequence = if let Some(l) = length {
            match self
                .seq
                .as_mut()
                .map(|reader| reader.next(l).map_err(Error::from))
            {
                None => None,
                Some(Ok(seq)) => Some(seq),
                Some(Err(e)) => return Some(Err(e)),
            }
        } else {
            None
        };
        // let com = match self.com.as_mut()
        //     .map(|block| {
        //         let mut buffer = Vec::new();
        //         block.read_until(0, &mut buffer).map_err(Error::from)?;
        //         String::from_utf8(buffer).map_err(Error::from)
        //     })
        // {

        // };

        self.n += 1;
        Some(Ok(Record {
            id,
            comment,
            sequence,
            quality: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCHIVE: &[u8] = include_bytes!("../../411476.SAMN00627058.naf");

    #[test]
    fn decoder() {
        use std::io::Write;

        let mut reader = Decoder::new(std::io::Cursor::new(ARCHIVE)).unwrap();
        let mut separator = reader.header.name_separator() as char;
        let mut line_length = reader.header.line_length() as usize;

        while let Some(result) = reader.next() {
            let record = result.unwrap();
            let sequence = record.sequence.unwrap();
        }
    }
}
