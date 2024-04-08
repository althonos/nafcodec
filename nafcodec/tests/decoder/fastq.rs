use nafcodec::Decoder;
use nafcodec::DecoderBuilder;
use nafcodec::Flag;
use nafcodec::Header;
use nafcodec::SequenceType;

const ARCHIVE: &[u8] = include_bytes!("../../../data/phix.naf");

fn check_header_flags(header: &Header) {
    assert!(header.flags().test(Flag::Quality));
    assert!(header.flags().test(Flag::Sequence));
    assert!(header.flags().test(Flag::Id));
    assert!(header.flags().test(Flag::Comment));
}

#[test]
fn decode_header() {
    let c = std::io::Cursor::new(ARCHIVE);
    let decoder = Decoder::new(c).unwrap();

    assert_eq!(decoder.header().name_separator(), ' ');
    assert_eq!(decoder.header().number_of_sequences(), 42);
    assert_eq!(decoder.header().sequence_type(), SequenceType::Dna);
    assert!(decoder.header().flags().test(Flag::Quality));
    assert!(decoder.header().flags().test(Flag::Sequence));
    assert!(decoder.header().flags().test(Flag::Id));
    assert!(decoder.header().flags().test(Flag::Comment));
}

#[test]
fn decode() {
    let c = std::io::Cursor::new(ARCHIVE);
    let mut decoder = Decoder::new(c).unwrap();
    check_header_flags(decoder.header());

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

#[test]
fn decode_no_id() {
    let mut decoder = DecoderBuilder::new().id(false).with_bytes(ARCHIVE).unwrap();
    check_header_flags(decoder.header());

    let r1 = decoder.next().unwrap().unwrap();
    assert!(r1.id.is_none());
    let r2 = decoder.next().unwrap().unwrap();
    assert!(r2.id.is_none());

    let records = decoder.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 40);
}

#[test]
fn decode_no_seq() {
    let mut decoder = DecoderBuilder::new()
        .sequence(false)
        .with_bytes(ARCHIVE)
        .unwrap();
    check_header_flags(decoder.header());

    let r1 = decoder.next().unwrap().unwrap();
    assert!(r1.sequence.is_none());
    let r2 = decoder.next().unwrap().unwrap();
    assert!(r2.sequence.is_none());

    let records = decoder.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 40);
}

#[test]
fn decode_no_comment() {
    let mut decoder = DecoderBuilder::new()
        .comment(false)
        .with_bytes(ARCHIVE)
        .unwrap();
    check_header_flags(decoder.header());

    let r1 = decoder.next().unwrap().unwrap();
    assert!(r1.comment.is_none());
    let r2 = decoder.next().unwrap().unwrap();
    assert!(r2.comment.is_none());

    let records = decoder.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 40);
}

#[test]
fn decode_no_quality() {
    let mut decoder = DecoderBuilder::new()
        .quality(false)
        .with_bytes(ARCHIVE)
        .unwrap();
    check_header_flags(decoder.header());

    let r1 = decoder.next().unwrap().unwrap();
    assert!(r1.quality.is_none());
    let r2 = decoder.next().unwrap().unwrap();
    assert!(r2.quality.is_none());

    let records = decoder.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(records.len(), 40);
}
