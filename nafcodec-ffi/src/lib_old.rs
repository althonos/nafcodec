//! `LibNAF` implemented in Rust
//! Authors: Jason Schlesinger, Kirill Kryukov
//! Acknowldgements to MEXT, DDBJ, and NIG for their support
//!
//! This library implements compression and decompression of the
//! NAF Filetype. It is provided as-is and with no warranty.
/* Base lib file exposed for consumption */
#![warn(
    missing_docs,
    clippy::unwrap_used,
    clippy::pedantic,
    clippy::expect_used
)]

use std::ffi::CStr;
use std::fs::File;
use std::os::raw::c_char;

use nafobj::NAFDataBlock;
use nafobj::NAFHeader;
use nafobj::NAFObject;
mod fasta;
mod files;
mod libennaf;
mod nafobj;
mod util;

// File IO Functions
/// Try to open a NAF file from a file name.
#[no_mangle]
pub unsafe extern "C" fn open_NAF(filename: *const c_char) -> Option<NAFObject> {
    if let Ok(obj) =
        unsafe { NAFObject::open_file(CStr::from_ptr(filename).to_str().unwrap().to_owned()) }
    {
        Some(obj)
    } else {
        None
    }
}

/// Open a FASTA file and create a NAFObject that populates
/// lazily given on how it is read.
#[no_mangle]
pub unsafe extern "C" fn open_FASTA(filename: *const c_char) -> NAFObject {
    let fasta_file: File = unsafe {
        let _filename = CStr::from_ptr(filename);
        File::open(_filename.to_str().unwrap()).expect("Cannot open file")
    };
    match fasta::naf_from_fasta(fasta_file, 1024) {
        Ok(naf_obj) => naf_obj,
        Err(_) => panic!("Could not parse FASTA"), // todo: log to error
    }
}

/// Open a FASTQ file and create a NAFObject that populates
/// lazily given how it is read.
#[no_mangle]
pub extern "C" fn open_FASTQ(_filename: String) -> NAFObject {
    todo!("Open a filename and return a readable file.");
}

/// Try to determine a file format heuristically from the
/// content of a file. This is only useful if the input isn't
/// available apriori, a scenario that should be avoided.
#[no_mangle]
pub extern "C" fn detect_input_format(_test_file: File) -> String {
    todo!("Detect file type from content alone");
}

/// Read the header data from a NAF Object.
#[no_mangle]
pub extern "C" fn read_header(naf: NAFObject) -> NAFHeader {
    *naf.read_header()
}

#[no_mangle]
pub extern "C" fn read_ids(naf: NAFObject) -> NAFDataBlock {
    match naf.load_ids() {
        Ok(ids) => ids,
        Err(_) => NAFDataBlock::new_ids(0),
    }
}

#[no_mangle]
pub extern "C" fn write_NAF(naf: NAFObject, outputfilename: String) {
    naf.save_to_file(outputfilename);
}

/// Represent "names" as a Vec of Strings
#[repr(C)]
pub struct Names {
    names: Vec<String>,
}

/// Represent a sequence as a Vec of Strings
#[repr(C)]
pub struct Sequence {
    sequence: Vec<String>,
}

/// Represents a FASTA file as a collection of names and sequences
#[repr(C)]
pub struct Fasta {
    names: Names,
    sequence: Sequence,
}

/// Represent a FastQ sequence as a collection of names, sequences, comments and qualities
#[repr(C)]
pub struct Fastq {
    names: Names,
    sequence: Sequence,
    comment: Names,
    qualities: Vec<i8>, // represents raw phred or solexa score
}

#[no_mangle]
pub fn read_fasta_entry(_fasta_file: File, _buffer_size: usize) -> &'static mut &'static mut Fasta {
    todo!("Read an entry from a FASTA file and return a pointer to the FASTA struct created");
}

#[no_mangle]
pub fn write_fasta_entry(_fasta_entry: &mut &mut Fasta) -> String {
    todo!("Write the String equivalent of the fasta entry")
}
