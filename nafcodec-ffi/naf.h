#ifndef LIBNAF
#define LIBNAF

#include <stdint.h>
#import <stdio.h>

typedef struct _custom_error Error;
typedef struct _naf_obj NafObj;

// File IO
NafObj open_NAF(char **s);
NafObj open_FASTA(char **s);
NafObj open_FASTQ(char **s);
char *detect_input_format(FILE);

// FASTA/Q Formatting
typedef struct _recod Record;
typedef struct _names Names;
typedef struct _sequence Sequence;
typedef struct _fasta Fasta;
typedef struct _fastq Fastq;

Record *read_next_record(*NafObj, int);
Sequence *read_next_sequence(*NafObj, int);
Fasta **read_FASTA(*NafObj, char[]);
Fastq **read_FASTQ(*NafObj, char[]);
int write_NAF(*NafObj, FILE);
int export_FASTA(*NafObj, FILE);
int export_FASTQ(*NafObj, FILE);

// NAF Formatting
typedef struct _header Header;
typedef struct _ids IDs;
typedef struct _lengths Lengths;
typedef struct _compressor Compressor;
typedef struct _mask Mask;

Header *read_header(*NafObj, int);
IDs *load_ids(*NafObj, int);
char **load_names(*NafObj, int);
Lengths *load_length(*NafObj, int);
char *load_mask(*NafObj, int);
Sequence *load_compressed_sequence(*NafObj, int);

// Util
long long read_number(FILE, int);
char *write_number(long long);
void die(char *);
void atexit();
char *put_magic_number();

// UnNAF methods
typedef struct _decompressor Decompressor;

char *print_ids(*NafObj);
char *print_names(*NafObj);
char *print_lengths(*NafObj);
char *print_total_length(*NafObj);
char *print_mask(*NafObj);
char *print_total_mask_length(*NafObj);
char *print_4bit(*NafObj);
char *print_dna(*NafObj);
char *print_fastq(*NafObj);

Decompressor *initialize_input_decompression();
Decompressor *initialize_quality_file_decompression();

#endif
