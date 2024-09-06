#![warn(
    missing_docs,
    clippy::unwrap_used,
    clippy::pedantic,
    clippy::expect_used
)]
use nafcodec::Flag;
use nafcodec::Flags;
use nafcodec::FormatVersion;
use nafcodec::Header;
use nafcodec::Record;
use nafcodec::SequenceType;
use nafcodec::Decoder;
use nafcodec::DecoderBuilder;
use nafcodec::Encoder;
use nafcodec::EncoderBuilder;
use nafcodec::Memory;
use nafcodec::Storage;
mod ennaf;
mod unnaf;
use safer_ffi::*;

use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn print_title(filename: String) -> String {
    todo!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // basic test of core funtion
    }
}
