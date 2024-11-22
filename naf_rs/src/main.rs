use clap::{Parser, Subcommand, ValueEnum};
use nafcodec::{SequenceType,Flag,Flags};
use std::fmt;

mod ennaf;
mod unnaf;

#[derive(Debug,Subcommand)]
enum Process {
    ENNAF ( EnnafArgs ),
    UNNAF ( UnnafArgs )
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum FileFormat {
    Fasta,
    Fastq
}

impl fmt::Display for FileFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{ 
        match self {
            FileFormat::Fasta => write!(f, "FASTA"),
            FileFormat::Fastq => write!(f, "FASTQ")
        }
    }
}

impl FileFormat {
    fn get_flags_for_format(&self) -> Flags {
        match self{
            FileFormat::Fasta => Flag::Sequence | Flag::Id | Flag::Comment,
            FileFormat::Fastq => Flag::Sequence | Flag::Id | Flag::Quality | Flag::Comment,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum SequenceTypeArg {
    Dna,
    Rna,
    Protein,
    Text
}

impl fmt::Display for SequenceTypeArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{ 
        match self {
            SequenceTypeArg::Dna => write!(f, "DNA"),
            SequenceTypeArg::Rna => write!(f, "RNA"),
            SequenceTypeArg::Protein => write!(f, "protein"),
            SequenceTypeArg::Text => write!(f, "text")
        }
    }
}

impl SequenceTypeArg {
    fn into_codec_type(&self) -> SequenceType {
        match self {
            SequenceTypeArg::Dna => SequenceType::Dna,
            SequenceTypeArg::Rna => SequenceType::Rna,
            SequenceTypeArg::Text => SequenceType::Text,
            SequenceTypeArg::Protein => SequenceType::Protein
        }
    }
}

#[derive(Parser,Debug)]
#[command(name="nafrs",version = "0.4", about = "Encode/decode NAF files", long_about = None)]
struct Args {
    #[command(subcommand)]
    process: Process
}

#[derive(Debug,Parser)]
struct EnnafArgs {
    #[arg(short,long,value_name="FILE",help="Write compressed output to FILE -- write to STDOUT if not specified")]
    output: Option<String>,
    #[arg(short='#',long,value_name="N",default_value_t=1,help="Use compression level N (from -131072 to 22)")]
    level: u16,
    #[arg(long,value_name="N",default_value_t=11,help="Use window size 2^N for sequence stream (from 10 to 31)")]
    long: u8,
    #[arg(long,value_name="DIR",env="TMP",help="Use DIR as temporary directory, (overrides TMP environment variable)")]
    temp_dir: Option<String>,
    #[arg(long,value_name="NAME",help="Use NAME as prefix for temporary files")]
    name: Option<String>,
    #[arg(long,value_name="TITLE",help="Use TITLE as dataset title")]
    title: Option<String>,
    #[arg(short,long,value_name="FORMAT",help="Input file type",default_value_t=FileFormat::Fasta)]
    format: FileFormat,
    #[arg(short,long,value_name="SEQTYPE",help="Input sequence type",default_value_t=SequenceTypeArg::Dna)]
    sequence: SequenceTypeArg,
    #[arg(long,default_value_t=false,help="Fail on unexpected input characters")]
    strict: bool,
    #[arg(long,value_name="N",default_value_t=80,help="Override line length to N")]
    line_length: u16,
    #[arg(long,default_value_t=false,help="Verbose mode")]
    verbose: bool,
    #[arg(long,default_value_t=false,help="Keep temporary files")]
    keep_temp_files: bool,
    #[arg(long,default_value_t=false,help="Don't store mask")]
    no_mask: bool,
    filename: String
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum UnnafOutput {
    Format, // file format & version
    PartList, // list of parts
    Sizes, // size of parts (original, compressed Band ratio)
    Number, // number of records
    Title, // frame title
    Ids, // record IDs (first part of FASTA name)
    Names, // full FASTA names
    Lengths, // sequence lengths
    TotalLength, // sum of sequence lengths
    Mask, // mask regions
    FourBit, // four-bit encoded sequences
    Seq, // all sequences concatenated (no newlines)
    Sequences, // sequences separated by newlines
    Fasta, // FASTA file output
    Fastq // FASTQ file output
}

impl fmt::Display for UnnafOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{ 
        match self {
            UnnafOutput::Format => write!(f,"Format"),
            UnnafOutput::PartList => write!(f,"Part List"),
            UnnafOutput::Sizes => write!(f,"Sizes"),
            UnnafOutput::Number => write!(f,"Number"),
            UnnafOutput::Title => write!(f,"Title"),
            UnnafOutput::Ids => write!(f,"IDs"),
            UnnafOutput::Names => write!(f,"Names"),
            UnnafOutput::Lengths => write!(f,"Lengths"),
            UnnafOutput::TotalLength => write!(f,"Total Lengths"),
            UnnafOutput::Mask => write!(f,"Mask"),
            UnnafOutput::FourBit => write!(f,"Four Bit"),
            UnnafOutput::Seq => write!(f,"Seq"),
            UnnafOutput::Sequences => write!(f,"Sequences"),
            UnnafOutput::Fasta => write!(f,"FASTA"),
            UnnafOutput::Fastq => write!(f,"FASTQ")
        }
    }
}

#[derive(Debug,Parser)]
struct UnnafArgs {
    #[arg(short,long,value_name="FILE",help="Write uncompressed output to FILE -- read to STDOUT if not specified")]
    output: Option<String>,
    #[arg(short='t',long,default_value_t=UnnafOutput::Fasta)]
    output_type: UnnafOutput,
    #[arg(long,value_name="N",default_value_t=80,help="Override line length to N")]
    line_length: u16,
    #[arg(long,default_value_t=false,help="Ignore Mask")]
    no_mask: bool,
    #[arg(long,default_value_t=false,help="Set STDOUT stream to binary mode")]
    binary_stdout: bool,
    #[arg(long,default_value_t=false,help="Set STDERR stream to binary mode")]
    binary_stderr: bool,
    filename: String
}

fn main() {
    let args=Args::parse();
    match args.process {
        Process::ENNAF (ennaf_args) => {
            ennaf::encode_file(&ennaf_args);
        }
        Process::UNNAF (unnaf_args) => {
            unnaf::decode_naf(&unnaf_args);
        }
    };
}
