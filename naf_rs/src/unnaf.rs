use nafcodec::{DecoderBuilder,Flag,Flags};

use std::path::Path;
use bio::io::{fasta,fastq};
use crate::UnnafArgs;

pub fn decode_naf(args: &UnnafArgs) {
    let flags = match args.output_type {
        crate::UnnafOutput::Format =>   Flags::new(),
        crate::UnnafOutput::PartList => Flags::new(),
        crate::UnnafOutput::Sizes =>    Flags::new(),
        crate::UnnafOutput::Number =>   Flags::from(Flag::Sequence),
        crate::UnnafOutput::Lengths =>  Flags::from(Flag::Length),
        crate::UnnafOutput::TotalLength => Flags::from(Flag::Length),
        crate::UnnafOutput::Title =>    Flags::from(Flag::Title),
        crate::UnnafOutput::Ids =>      Flags::from(Flag::Id),
        crate::UnnafOutput::Names =>    Flags::from(Flag::Comment),
        crate::UnnafOutput::Mask =>     Flags::from(Flag::Mask),
        crate::UnnafOutput::FourBit | 
          crate::UnnafOutput::Seq |
          crate::UnnafOutput::Sequences => Flags::from(Flag::Sequence),
        crate::UnnafOutput::Fasta => Flag::Comment | Flag::Sequence | {if args.no_mask {Flag::Mask} else {Flag::Sequence}},
        crate::UnnafOutput::Fastq => Flag::Comment | Flag::Sequence | Flag::Quality | {if args.no_mask {Flag::Mask} else {Flag::Sequence}},
    };
    let filepath = Path::new(&args.filename);
    let mut decoder = DecoderBuilder::from_flags(flags).with_path(filepath).unwrap();
    // FIXME: Write to args.output instead of using println!
    match args.output_type{
        crate::UnnafOutput::Format => {
            let header = decoder.header();
            println!("{:?} sequences in NAF format {:?}",header.sequence_type(),header.format_version());
        },
        crate::UnnafOutput::PartList => println!("{:?}",decoder.header().flags()),
        crate::UnnafOutput::Sizes => {
            let all_flags = decoder.header().flags();
            let sizes_decoder = DecoderBuilder::from_flags(all_flags).sizes_from_path(filepath).unwrap();
            for size in sizes_decoder {
                println!("{}",size);
            }
        },
        crate::UnnafOutput::Lengths => println!("{:?}",decoder.lengths()),
        crate::UnnafOutput::TotalLength => println!("{:?}",decoder.lengths().iter().sum::<u64>()),
        crate::UnnafOutput::Number => println!("{:?}",decoder.lengths().len()),
        crate::UnnafOutput::Title => println!("{}",decoder.title().unwrap()),
        crate::UnnafOutput::Ids => {
            for record in decoder {
                println!("{}",record.unwrap().id.unwrap());
            }
        },
        crate::UnnafOutput::Names => {
            for record in decoder {
                if let Ok(ok_rec) = record {
                    println!("{} {}",ok_rec.id.unwrap(),ok_rec.comment.unwrap());
                }
            }
        },
        crate::UnnafOutput::Mask => todo!(),
        crate::UnnafOutput::FourBit => todo!(),
        crate::UnnafOutput::Seq => {
            for record in decoder {
                if let Ok(ok_rec) = record {
                    print!("{}",std::str::from_utf8(&ok_rec.sequence.unwrap()).unwrap());
                }
            }
        },
        crate::UnnafOutput::Sequences => {
            for record in decoder {
                if let Ok(ok_rec) = record {
                    println!("{}",std::str::from_utf8(&ok_rec.sequence.unwrap()).unwrap());
                }
            }
        },
        crate::UnnafOutput::Fasta => {
            for record in decoder {
                if let Ok(ok_rec) = record {
                    let seq = ok_rec.sequence.unwrap();
                    let id = ok_rec.id.unwrap().clone();
                    let comment = ok_rec.comment.as_deref().clone();
                    let fasta_record = fasta::Record::with_attrs(
                        &id,
                        comment,
                        &seq);
                    // FIXME: does not wrap lines
                    print!("{}",fasta_record);
                }

            }
        },
        crate::UnnafOutput::Fastq => 
            for record in decoder {
                if let Ok(ok_rec) = record {
                    let seq = ok_rec.sequence.unwrap();
                    let id = ok_rec.id.unwrap().clone();
                    let comment = ok_rec.comment.as_deref().clone();
                    let qual = ok_rec.quality.expect("FASTQ output requested, but input has no qualities");
                    let fastq_record = fastq::Record::with_attrs(
                        &id,
                        comment,
                        &seq,
                        qual.as_bytes());
                    // FIXME: does not wrap lines
                    print!("{}",fastq_record);
                }

            }
    }
    
}
