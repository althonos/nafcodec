use nafcodec::Decoder;
use nafcodec::SequenceType;

#[test]
fn decode() {
    const ARCHIVE: &[u8] = include_bytes!("../../../data/LuxC.naf");

    let c = std::io::Cursor::new(ARCHIVE);
    let mut decoder = Decoder::new(c).unwrap();

    assert_eq!(decoder.header().name_separator(), ' ');
    assert_eq!(decoder.header().number_of_sequences(), 12);
    assert_eq!(decoder.header().line_length(), 60);
    assert_eq!(decoder.header().sequence_type(), SequenceType::Protein);

    let r1 = decoder.next().unwrap().unwrap();
    assert!(r1.id.is_some());
    assert!(r1.sequence.is_some());

    let seq = r1.sequence.unwrap();
    assert_eq!(seq.len(), 488);
}
