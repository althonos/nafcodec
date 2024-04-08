//! Common data types for this crate.

// --- MaskUnit ----------------------------------------------------------------

use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::BitOrAssign;

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

/// The value of a single `Flag` inside header [`Flags`].
#[repr(u8)]
pub enum Flag {
    /// A flag indicating sequence qualities are stored in the archive.
    Quality = 0x1,
    /// A flag indicating sequences are stored in the archive.
    Sequence = 0x2,
    /// A flag indicating sequence masks are stored in the archive.
    Mask = 0x4,
    /// A flag indicating sequence lengths are stored in the archive.
    Lengths = 0x8,
    /// A flag indicating sequence comments are stored in the archive.
    Comments = 0x10,
    /// A flag indicating sequence identifiers are stored in the archive.
    Ids = 0x20,
    /// A flag indicating the archive has a title.
    Title = 0x40,
    /// A flag reserved for future extension of the format.
    Extended = 0x80,
}

impl BitAnd<Flag> for u8 {
    type Output = u8;
    fn bitand(self, rhs: Flag) -> Self::Output {
        self.bitand(rhs as u8)
    }
}

impl BitOr<Flag> for Flag {
    type Output = u8;
    fn bitor(self, rhs: Flag) -> Self::Output {
        (self as u8).bitor(rhs)
    }
}

impl BitOr<Flag> for u8 {
    type Output = u8;
    fn bitor(self, rhs: Flag) -> Self::Output {
        self.bitor(rhs as u8)
    }
}

/// The flags for optional content blocks inside a NAF archive.
#[derive(Debug, Clone, Copy)]
pub struct Flags(u8);

impl Flags {
    /// Create new `Flags` from the given value.
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn has_quality(&self) -> bool {
        (self.0 & Flag::Quality) != 0
    }

    pub fn has_sequence(&self) -> bool {
        (self.0 & Flag::Sequence) != 0
    }

    pub fn has_mask(&self) -> bool {
        (self.0 & Flag::Mask) != 0
    }

    pub fn has_lengths(&self) -> bool {
        (self.0 & Flag::Lengths) != 0
    }

    pub fn has_comments(&self) -> bool {
        (self.0 & Flag::Comments) != 0
    }

    pub fn has_ids(&self) -> bool {
        (self.0 & Flag::Ids) != 0
    }

    pub fn has_title(&self) -> bool {
        (self.0 & Flag::Title) != 0
    }

    pub fn has_extended_format(&self) -> bool {
        (self.0 & Flag::Extended) != 0
    }
}

impl Default for Flags {
    fn default() -> Self {
        Self::new(0)
    }
}

impl From<Flags> for u8 {
    fn from(flags: Flags) -> Self {
        flags.0
    }
}

impl BitOr<Flag> for Flags {
    type Output = Flags;
    fn bitor(self, rhs: Flag) -> Self::Output {
        Flags(self.0.bitor(rhs as u8))
    }
}

impl BitOrAssign<Flag> for Flags {
    fn bitor_assign(&mut self, rhs: Flag) {
        self.0 = self.0 | rhs;
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
    /// Get the flags of the archive header.
    pub fn flags(&self) -> Flags {
        self.flags
    }

    /// Get the default line length stored in the archive.
    pub fn line_length(&self) -> u64 {
        self.line_length
    }

    /// Get the name separator used in the archive.
    pub fn name_separator(&self) -> char {
        self.name_separator
    }

    /// Get the number of sequences stored in the archive.
    pub fn number_of_sequences(&self) -> u64 {
        self.number_of_sequences
    }

    /// Get the type of sequences stored in the archive.
    pub fn sequence_type(&self) -> SequenceType {
        self.sequence_type
    }

    /// Get the archive format version.
    pub fn format_version(&self) -> FormatVersion {
        self.format_version
    }
}

impl Default for Header {
    fn default() -> Self {
        Header {
            format_version: FormatVersion::V1,
            sequence_type: SequenceType::Dna,
            flags: Flags::default(),
            name_separator: ' ',
            line_length: 60,
            number_of_sequences: 0,
        }
    }
}
