extern crate naf;

use naf::data::SequenceType;
use naf::decoder::Decoder;

#[test]
fn decode() {
    const ARCHIVE: &[u8] = include_bytes!("../data/NZ_AAEN01000029.naf");

    let c = std::io::Cursor::new(ARCHIVE);
    let mut decoder = Decoder::new(c).unwrap();

    assert_eq!(decoder.header.name_separator(), ' ');
    assert_eq!(decoder.header.number_of_sequences(), 30);
    assert_eq!(decoder.header.line_length(), 80);
    assert_eq!(decoder.header.sequence_type(), SequenceType::Dna);    


    let r1 = decoder.next().unwrap().unwrap();
    assert_eq!(r1.id.unwrap(), "NZ_AAEN01000029.1");
    assert_eq!(r1.comment.unwrap(), "Bacillus anthracis str. CNEVA-9066 map unlocalized plasmid pXO1 cont2250, whole genome shotgun sequence");
    let seq = r1.sequence.unwrap();
    assert_eq!(seq.len(), 182777);
    assert_eq!(seq.chars().filter(|&x| x == 'A').count(), 62115);
    assert_eq!(seq.chars().filter(|&x| x == 'C').count(), 28747);
    assert_eq!(seq.chars().filter(|&x| x == 'G').count(), 30763);
    assert_eq!(seq.chars().filter(|&x| x == 'T').count(), 61152);

    let r2 = decoder.next().unwrap().unwrap();
    assert_eq!(r2.id.unwrap(), "NZ_AAEN01000030.3");
    assert_eq!(r2.comment.unwrap(), "Bacillus anthracis str. CNEVA-9066 map unlocalized plasmid pXO2 cont2251, whole genome shotgun sequence");

    let records = decoder.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 28);
}