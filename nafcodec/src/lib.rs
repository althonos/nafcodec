#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "nightly", feature(seek_stream_len))]
#![cfg_attr(feature = "nightly", feature(iter_advance_by))]

mod data;
mod decoder;

pub mod error;

pub use self::data::Flags;
pub use self::data::FormatVersion;
pub use self::data::Header;
pub use self::data::Record;
pub use self::data::SequenceType;
pub use self::decoder::Decoder;
pub use self::decoder::DecoderBuilder;
