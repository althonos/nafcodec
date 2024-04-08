#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "nightly", feature(seek_stream_len))]
#![cfg_attr(feature = "nightly", feature(iter_advance_by))]

mod data;
mod decoder;
mod encoder;

pub mod error;

pub use self::data::Flag;
pub use self::data::Flags;
pub use self::data::FormatVersion;
pub use self::data::Header;
pub use self::data::Record;
pub use self::data::SequenceType;
pub use self::decoder::Decoder;
pub use self::decoder::DecoderBuilder;
pub use self::encoder::Encoder;
pub use self::encoder::EncoderBuilder;
pub use self::encoder::Memory;
pub use self::encoder::Storage;

/// The reference counter type used to share the stream.
#[cfg(feature = "arc")]
type Rc<T> = std::sync::Arc<T>;
#[cfg(not(feature = "arc"))]
type Rc<T> = std::rc::Rc<T>;
