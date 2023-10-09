#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "nightly", feature(seek_stream_len))]
#![cfg_attr(feature = "nightly", feature(iter_advance_by))]

mod decoder;

pub mod data;
pub mod error;

pub use self::decoder::Decoder;
pub use self::decoder::DecoderBuilder;
