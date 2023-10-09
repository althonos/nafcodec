use nafcodec::Decoder;
use nafcodec::SequenceType;

#[test]
fn decode() {
    const ARCHIVE: &[u8] = include_bytes!("../../data/phix.naf");

    let c = std::io::Cursor::new(ARCHIVE);
    let mut decoder = Decoder::new(c).unwrap();

    assert_eq!(decoder.header().name_separator(), ' ');
    assert_eq!(decoder.header().number_of_sequences(), 42);
    assert_eq!(decoder.header().sequence_type(), SequenceType::Dna);
    assert!(decoder.header().flags().has_quality());
    assert!(decoder.header().flags().has_sequence());

    let r1 = decoder.next().unwrap().unwrap();
    assert_eq!(r1.id.unwrap(), "SRR1377138.1");
    assert_eq!(
        r1.comment.unwrap(),
        "a comment that should not be included in the SAM output"
    );
    let seq = r1.sequence.unwrap();
    assert!(seq.starts_with("NGCTCTTAAACCTGCTATTGAGGCTTGTGGCATTTC"));
    let qual = r1.quality.unwrap();
    assert!(qual.starts_with("#8CCCGGGGGGGGGGGGGGGGGGGGGGGGGG"));

    let r2 = decoder.next().unwrap().unwrap();
    assert_eq!(r2.id.unwrap(), "SRR1377138.2");
    assert_eq!(r2.comment.unwrap(), "some lowercase nucleotides");

    let records = decoder.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 40);
}
