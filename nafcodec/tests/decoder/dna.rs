use nafcodec::Decoder;
use nafcodec::DecoderBuilder;
use nafcodec::SequenceType;

const GENOME: &[u8] = include_bytes!("../../../data/NZ_AAEN01000029.naf");
const MASKED: &[u8] = include_bytes!("../../../data/masked.naf");

#[test]
fn decode() {
    let c = std::io::Cursor::new(GENOME);
    let mut decoder = Decoder::new(c).unwrap();

    assert_eq!(decoder.header().name_separator(), ' ');
    assert_eq!(decoder.header().number_of_sequences(), 30);
    assert_eq!(decoder.header().line_length(), 80);
    assert_eq!(decoder.header().sequence_type(), SequenceType::Dna);

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

#[test]
fn mask() {
    let c = std::io::Cursor::new(MASKED);
    let mut decoder = Decoder::new(c).unwrap();

    assert_eq!(decoder.header().name_separator(), ' ');
    assert_eq!(decoder.header().number_of_sequences(), 2);
    assert_eq!(decoder.header().line_length(), 50);
    assert_eq!(decoder.header().sequence_type(), SequenceType::Dna);

    let r1 = decoder.next().unwrap().unwrap();
    assert_eq!(r1.id.unwrap(), "test1");
    let seq = r1.sequence.unwrap();
    assert!(seq[..657].chars().all(|x| x.is_uppercase()));
    assert!(seq[657..676].chars().all(|x| x.is_lowercase()));
    assert!(seq[676..1311].chars().all(|x| x.is_uppercase()));
    assert!(seq[1311..1350].chars().all(|x| x.is_lowercase()));

    let r2 = decoder.next().unwrap().unwrap();
    assert_eq!(r2.id.unwrap(), "test2");
    let seq = r2.sequence.unwrap();
    assert!(seq[..525].chars().all(|x| x.is_uppercase()));
    assert!(seq[525..621].chars().all(|x| x.is_lowercase()));
    assert!(seq[621..720].chars().all(|x| x.is_uppercase()));
    assert!(seq[720..733].chars().all(|x| x.is_lowercase()));

    assert!(decoder.next().is_none());
}

#[test]
fn force_nomask() {
    let mut decoder = DecoderBuilder::new()
        .mask(false)
        .with_bytes(MASKED)
        .unwrap();

    assert_eq!(decoder.header().name_separator(), ' ');
    assert_eq!(decoder.header().number_of_sequences(), 2);
    assert_eq!(decoder.header().line_length(), 50);
    assert_eq!(decoder.header().sequence_type(), SequenceType::Dna);

    let r1 = decoder.next().unwrap().unwrap();
    assert_eq!(r1.id.unwrap(), "test1");
    let seq = r1.sequence.unwrap();
    assert!(seq[..].chars().all(|x| x.is_uppercase()));

    let r2 = decoder.next().unwrap().unwrap();
    assert_eq!(r2.id.unwrap(), "test2");
    let seq = r2.sequence.unwrap();
    assert!(seq[..].chars().all(|x| x.is_uppercase()));

    assert!(decoder.next().is_none());
}
