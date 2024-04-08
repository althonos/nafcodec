use nom::IResult;

use crate::data::Flag;
use crate::data::Flags;
use crate::data::FormatVersion;
use crate::data::Header;
use crate::data::SequenceType;

fn is_printable(&byte: &u8) -> bool {
    (0x20..=0x7E).contains(&byte)
}

pub fn byte(i: &[u8]) -> IResult<&[u8], u8> {
    let (i, n) = nom::bytes::streaming::take(1usize)(i)?;
    Ok((i, n[0]))
}

// pub fn variable_length(i: &[u8]) -> IResult<&[u8], VariableLength> {
//     let (i, n) = nom::bytes::streaming::take_while(|byte: u8| (byte & 0x80) != 0)(i)?;
//     let (i, x) = self::byte(i)?;
//     let mut limbs = n.to_vec();
//     limbs.push(x);
//     Ok((i, VariableLength::new(limbs)))
// }

pub fn variable_u64(i: &[u8]) -> IResult<&[u8], u64> {
    let (i1, limbs) = nom::bytes::streaming::take_while(|byte: u8| (byte & 0x80) != 0)(i)?;
    let (i2, last) = self::byte(i1)?;

    let mut num = 0;
    let mut basis = 1;

    num += ((last & 0x7F) as u64) * basis;
    basis *= 128;

    for &limb in limbs.iter().rev() {
        if let Some(x) = num.checked_add(((limb & 0x7F) as u64) * basis) {
            num = x;
            basis *= 128;
        } else {
            let error = nom::error::Error::new(i, nom::error::ErrorKind::TooLarge);
            return Err(nom::Err::Failure(error));
        }
    }

    Ok((i2, num))
}

pub fn format_descriptor(i: &[u8]) -> IResult<&[u8], &[u8]> {
    nom::bytes::streaming::tag([0x01, 0xF9, 0xEC])(i)
}

pub fn format_version(i: &[u8]) -> IResult<&[u8], FormatVersion> {
    match nom::combinator::verify(self::byte, |&byte: &u8| byte == 1 || byte == 2)(i) {
        Err(e) => Err(e),
        Ok((i, 1)) => Ok((i, FormatVersion::V1)),
        Ok((i, 2)) => Ok((i, FormatVersion::V2)),
        _ => unreachable!(),
    }
}

pub fn sequence_type(i: &[u8]) -> IResult<&[u8], SequenceType> {
    match nom::combinator::verify(self::byte, |&byte: &u8| byte <= 0x03)(i) {
        Err(e) => Err(e),
        Ok((i, 0)) => Ok((i, SequenceType::Dna)),
        Ok((i, 1)) => Ok((i, SequenceType::Rna)),
        Ok((i, 2)) => Ok((i, SequenceType::Protein)),
        Ok((i, 3)) => Ok((i, SequenceType::Text)),
        _ => unreachable!(),
    }
}

pub fn flags(i: &[u8]) -> IResult<&[u8], Flags> {
    self::byte(i).map(|(i, x)| {
        let mut flags = Flags::new();
        for flag in Flag::values() {
            if x & flag.as_byte() != 0 {
                flags.set(*flag);
            }
        }
        (i, flags)
    })
}

pub fn name_separator(i: &[u8]) -> IResult<&[u8], char> {
    nom::combinator::verify(self::byte, self::is_printable)(i).map(|(i, c)| (i, c as char))
}

pub fn line_length(i: &[u8]) -> IResult<&[u8], u64> {
    self::variable_u64(i)
}

pub fn number_of_sequences(i: &[u8]) -> IResult<&[u8], u64> {
    self::variable_u64(i)
}

pub fn header(i: &[u8]) -> IResult<&[u8], Header> {
    let (i, _) = format_descriptor(i)?;
    let (i, fmt) = format_version(i)?;
    let (i, seqty) = match fmt {
        FormatVersion::V1 => (i, SequenceType::Dna),
        FormatVersion::V2 => sequence_type(i)?,
    };
    let (i, f) = flags(i)?;
    let (i, sep) = name_separator(i)?;
    let (i, ll) = line_length(i)?;
    let (i, nseq) = number_of_sequences(i)?;
    Ok((
        i,
        Header {
            format_version: fmt,
            sequence_type: seqty,
            flags: f,
            name_separator: sep,
            line_length: ll,
            number_of_sequences: nseq,
        },
    ))
}

pub fn title(i: &[u8]) -> IResult<&[u8], &str> {
    let (i, size) = self::variable_u64(i)?;
    if size > (usize::MAX as u64) {
        return IResult::Err(nom::Err::Failure(nom::error::Error::new(
            i,
            nom::error::ErrorKind::TooLarge,
        )));
    }
    let (i, text) = nom::combinator::map_res(
        nom::bytes::streaming::take(size as usize),
        std::str::from_utf8,
    )(i)?;
    Ok((i, text))
}

mod tests {

    #[test]
    fn header() {
        const HEADER: [u8; 8] = [0x01, 0xF9, 0xEC, 0x01, 0x3E, 0x20, 0x3C, 0x20];
        let (i, h) = super::header(&HEADER[..]).unwrap();
        assert_eq!(h.name_separator(), ' ');
        assert_eq!(h.line_length(), 60);
        assert_eq!(h.number_of_sequences(), 32);
        assert_eq!(i, b"");
    }
}
