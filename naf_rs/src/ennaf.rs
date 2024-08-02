use nafcodec::{EncoderBuilder,Memory,Flag,Record};
use std::borrow::Cow;
use std::fs::File;
use std::io::BufWriter;
use std::str;
use bio::io::{fasta,fastq};
use crate::EnnafArgs;
use crate::FileFormat;
use crate::SequenceTypeArg;

pub fn encode_file(args: &EnnafArgs) {
    let all_flags = args.format.get_flags_for_format() | Flag::Length |
    { if !args.no_mask && (args.sequence == SequenceTypeArg::Dna || args.sequence == SequenceTypeArg::Rna)  
        {Flag::Mask} 
      else 
        {Flag::Length}
    } |
    { if let Some(_) = args.title 
        {Flag::Title} 
      else 
        {Flag::Length}
    };  
    let mut encoder = EncoderBuilder::from_flags(args.sequence.into_codec_type(), all_flags).with_storage(Memory).unwrap();
    if let Some(title) = &args.title {
        encoder.push_title(title);
    }
    match args.format {
        FileFormat::Fasta => {
            let fasta_file = File::open(args.filename.to_owned()).unwrap();
            let fasta_reader = fasta::Reader::new(fasta_file);
            for results in fasta_reader.records() {
                let record = results.unwrap(); 
                let r = Record{
                    id:Some(Cow::from(record.id())),
                    comment:{if let Some(desc) = record.desc() {
                                Some(Cow::from(desc))
                            } else {
                                None
                            }},
                    sequence:Some(Cow::from(record.seq().to_vec())),
                    length: Some(record.seq().len() as u64),
                    quality: None
                };
                println!{"Parsed record {:?}",r};
                if let Err(e) = encoder.push(&r) { 
                    panic!{"Could not push record {:?} to encoder {}",r,e};
                }
            }
        },
        FileFormat::Fastq => {
            let fastq_file = File::open(args.filename.to_owned()).unwrap();
            let fastq_reader = fastq::Reader::new(fastq_file);
            for results in fastq_reader.records() {
                let record = results.unwrap();
                let r = Record{
                    id:Some(Cow::from(record.id())),
                    comment:{if let Some(desc) = record.desc() {
                                Some(Cow::from(desc))
                            } else {
                                None
                            }},
                    sequence:Some(Cow::from(record.seq())),
                    length: Some(record.seq().len() as u64),
                    quality: Some(Cow::from(str::from_utf8(record.qual()).unwrap()))
                };
                if let Err(e) = encoder.push(&r) {
                    panic!{"Could not push record {:?} to encoder {}",r,e};
                }
            }
        }
    }
    match &args.output {
        Some(outfile) => encoder.write(BufWriter::new(File::create(outfile.to_owned()).unwrap())),

        None => encoder.write(BufWriter::new(std::io::stdout()))
    };    
}


