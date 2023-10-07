#![doc = include_str!("../README.md")]

mod decoder;

pub mod data;
pub mod error;

pub use self::decoder::Decoder;
pub use self::decoder::DecoderBuilder;
