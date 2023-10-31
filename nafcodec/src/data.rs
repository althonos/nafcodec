//! Common data types for this crate.

// --- MaskUnit ----------------------------------------------------------------

/// A single masked unit with associated status decoded from the mask block.
#[derive(Debug, Clone, PartialEq)]
pub enum MaskUnit {
    Masked(u64),
    Unmasked(u64),
}

// --- Record ------------------------------------------------------------------

/// A single sequence record from a Nucleotide Archive Format file.
///
/// ## Quality
///
/// If set, the quality string length should be equal to the sequence
/// string length, and to the record length. Since the data is compressed
/// as raw text, it could contain other sort of annotation, such as RNA
/// secondary structure in dot-bracket notation, or protein secondary
/// structure.
///
#[derive(Debug, Clone, Default)]
pub struct Record {
    /// The record identifier (accession number).
    pub id: Option<String>,
    /// The record comment (description).
    pub comment: Option<String>,
    /// The record sequence.
    pub sequence: Option<String>,
    /// The record quality string.
    pub quality: Option<String>,
    /// The record sequence length.
    pub length: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum FormatVersion {
    #[default]
    V1 = 1,
    V2 = 2,
}

/// The type of sequence stored in a Nucleotide Archive Format file.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SequenceType {
    #[default]
    Dna = 0,
    Rna = 1,
    Protein = 2,
    Text = 3,
}

impl SequenceType {
    /// Check whether the sequence type is a nucleotide type.
    #[inline]
    pub fn is_nucleotide(&self) -> bool {
        match self {
            Self::Dna | Self::Rna => true,
            Self::Protein | Self::Text => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Flags(u8);

impl Flags {
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn has_quality(&self) -> bool {
        (self.0 & 0x1) != 0
    }

    pub fn has_sequence(&self) -> bool {
        (self.0 & 0x2) != 0
    }

    pub fn has_mask(&self) -> bool {
        (self.0 & 0x4) != 0
    }

    pub fn has_lengths(&self) -> bool {
        (self.0 & 0x8) != 0
    }

    pub fn has_comments(&self) -> bool {
        (self.0 & 0x10) != 0
    }

    pub fn has_ids(&self) -> bool {
        (self.0 & 0x20) != 0
    }

    pub fn has_title(&self) -> bool {
        (self.0 & 0x40) != 0
    }

    pub fn has_extended_format(&self) -> bool {
        (self.0 & 0x80) != 0
    }
}

impl Default for Flags {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Into<u8> for Flags {
    fn into(self) -> u8 {
        self.0
    }
}

/// The header section of a Nucleotide Archive Format file.
///
/// Headers are the only mandatory section of NAF files, and contain
/// metadata about the stored sequences, as well as some metadata for
/// the formatting the records during decompression.
///
#[derive(Debug, Clone)]
pub struct Header {
    pub(crate) format_version: FormatVersion,
    pub(crate) sequence_type: SequenceType,
    pub(crate) flags: Flags,
    pub(crate) name_separator: char,
    pub(crate) line_length: u64,
    pub(crate) number_of_sequences: u64,
}

impl Header {
    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn line_length(&self) -> u64 {
        self.line_length
    }

    pub fn name_separator(&self) -> char {
        self.name_separator
    }

    pub fn number_of_sequences(&self) -> u64 {
        self.number_of_sequences
    }

    pub fn sequence_type(&self) -> SequenceType {
        self.sequence_type
    }

    pub fn format_version(&self) -> FormatVersion {
        self.format_version
    }
}

impl Default for Header {
    fn default() -> Self {
        Header {
            format_version: FormatVersion::V2,
            sequence_type: SequenceType::Dna,
            flags: Flags::default(),
            name_separator: ' ',
            line_length: 60,
            number_of_sequences: 0,
        }
    }
}
