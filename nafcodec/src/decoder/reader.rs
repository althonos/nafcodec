use std::ffi::CString;
use std::io::BufRead;
use std::io::ErrorKind;
use log::{warn,debug};

use crate::data::MaskUnit;
use crate::data::SequenceType;
#[cfg(all(target_arch="x86_64",feature="simd"))]
use core::arch::x86_64::{
    __cpuid_count,
    _mm_storeu_si128,
    _mm_set_epi8,
    _mm_set1_epi32,
    _mm_set1_epi8,
    _mm_shuffle_epi8,
    _mm_srlv_epi32,
    __m128i,
    _mm_loadu_si128,
    _mm_unpackhi_epi8,
    _mm_unpacklo_epi8,
    _mm_and_si128,
};
// --- CStringReader -----------------------------------------------------------

#[derive(Debug)]
pub struct CStringReader<R: BufRead> {
    reader: R,
}

impl<R: BufRead> CStringReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: BufRead> Iterator for CStringReader<R> {
    type Item = Result<CString, std::io::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = Vec::new();
        match self.reader.read_until(0, &mut buffer) {
            Ok(0) => None,
            Err(e) => Some(Err(e)),
            Ok(_) => Some(Ok(CString::from_vec_with_nul(buffer)
                .expect("buffer should contain a single nul byte"))),
        }
    }
}

// --- LengthReader ------------------------------------------------------------

#[derive(Debug)]
pub struct LengthReader<R: BufRead> {
    reader: R,
}

impl<R: BufRead> LengthReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: BufRead> Iterator for LengthReader<R> {
    type Item = Result<u64, std::io::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut n = 0u64;
        let mut x = u32::MAX;
        let mut buffer = [0u8; 4];

        while x == u32::MAX {
            if let Err(e) = self.reader.read_exact(&mut buffer[..]) {
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => return None,
                    _ => return Some(Err(e)),
                }
            }
            x = nom::number::complete::le_u32::<&[u8], nom::error::Error<&[u8]>>(&buffer[..])
                .unwrap()
                .1;
            n += x as u64;
        }

        Some(Ok(n))
    }
}

// --- SequenceReader ----------------------------------------------------------


#[derive(Debug)]
pub struct SequenceReader<R: BufRead> {
    reader: R,
    ty: SequenceType,
    cache: Option<char>,
}

impl<R: BufRead> SequenceReader<R> {
    pub fn new(reader: R, ty: SequenceType) -> Self {
        Self {
            reader,
            ty,
            cache: None,
        }
    }

    pub fn next(&mut self, length: u64) -> Result<String, std::io::Error> {
        let l = length as usize;
        if self.ty.is_nucleotide() {
            let mut sequence = String::with_capacity(l);
            if l > 0 {
                if let Some(_) = self.cache {
                    match self.cache.take() {
                        Some(cache_take) => sequence.push(cache_take),
                        None => return Err(std::io::Error::new(ErrorKind::UnexpectedEof,"Could not find next record in cache"))
                    }
                }
            }
            while sequence.len() < l {
                match self.ty {
                    SequenceType::Dna => self.read_nucleotide::<'T'>(l, &mut sequence)?,
                    SequenceType::Rna => self.read_nucleotide::<'U'>(l, &mut sequence)?,
                    _ => unreachable!(),
                }
            }
            Ok(sequence)
        } else {
            let mut sequence = Vec::with_capacity(l);
            while sequence.len() < l {
                self.read_text(l, &mut sequence)?;
            }
            String::from_utf8(sequence)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
    }

    fn read_text(&mut self, length: usize, sequence: &mut Vec<u8>) -> Result<(), std::io::Error> {
        let buffer = self.reader.fill_buf()?;
        let n_to_copy = buffer.len().min(length - sequence.len());
        sequence.extend_from_slice(&buffer[..n_to_copy]);
        self.reader.consume(n_to_copy);
        Ok(())
    }

    fn read_nucleotide<const T: char>(
        &mut self,
        length: usize,
        sequence: &mut String,
    ) -> Result<(), std::io::Error> {
        let buffer = self.reader.fill_buf()?;

        let rem = length - sequence.len();
        let n = buffer.len().min(rem/2);

        // decode the bulk of the characters
        cfg_if::cfg_if!{
            if #[cfg(feature="simd")] {
                let mut offset = 0;
                for i in 0..(n-(n%16))/16{
                    let mut simd_buf: [u8;16] = [0;16];
                    for (j,x) in buffer[i*16..(i+1)*16].iter().take(16).enumerate() {
                        simd_buf[j] = *x;
                    }
                    if let Ok(seq_buf) = Self::decode_simd::<T>(simd_buf) {
                        sequence.push_str(String::from_utf8(seq_buf.iter().map(|x| *x).collect::<Vec<u8>>()).unwrap().as_str());
                        offset += 16;
                    } else {
                        warn!("SIMD Decoding failed, using LUT");
                        break; 
                    }
                }
                debug!("{:?} bytes of {:?} read, switching to LUT to parse next {}",offset,rem,buffer[offset..n].len());
                for x in buffer[offset..n].iter().take(n-offset) {
                    let c = Self::decode_lut::<T>(*x);
                    sequence.push(c[0]);
                    sequence.push(c[1]);
                }
            } else if #[cfg(feature="lut")] {
                warn!("Parsing sequence using LUT");
                for x in buffer.iter().take(n) {
                    let c = Self::decode_lut::<T>(*x);
                    sequence.push(c[0]);
                    sequence.push(c[1]);
                }
            } else {
                for x in buffer.iter().take(n) {
                    let c1 = Self::decode::<T>(x & 0x0F);
                    sequence.push(c1);
                    let c2 = Self::decode::<T>(x >> 4);
                    sequence.push(c2);
                }
            }
        }

        if n < buffer.len() && sequence.len() == length - 1 {
            let c1 = Self::decode::<T>(buffer[n] & 0x0F);
            sequence.push(c1);
            let c2 = Self::decode::<T>(buffer[n] >> 4);
            self.cache = Some(c2);
            self.reader.consume(n + 1);
        } else {
            self.reader.consume(n);
        }

        Ok(())
    }

    #[cfg(feature="lut")]
    #[inline]
    fn decode_lut<const T: char>(c: u8) -> [char;2] {
        const CHARS_LUT_U: [[char;2];256] = [
            ['-','-'], ['U','-'], ['G','-'], ['K','-'], ['C','-'], ['Y','-'], ['S','-'],
            ['B','-'], ['A','-'], ['W','-'], ['R','-'], ['D','-'], ['M','-'], ['H','-'],
            ['V','-'], ['N','-'], ['-','U'], ['U','U'], ['G','U'], ['K','U'], ['C','U'],
            ['Y','U'], ['S','U'], ['B','U'], ['A','U'], ['W','U'], ['R','U'], ['D','U'],
            ['M','U'], ['H','U'], ['V','U'], ['N','U'], ['-','G'], ['U','G'], ['G','G'],
            ['K','G'], ['C','G'], ['Y','G'], ['S','G'], ['B','G'], ['A','G'], ['W','G'],
            ['R','G'], ['D','G'], ['M','G'], ['H','G'], ['V','G'], ['N','G'], ['-','K'],
            ['U','K'], ['G','K'], ['K','K'], ['C','K'], ['Y','K'], ['S','K'], ['B','K'],
            ['A','K'], ['W','K'], ['R','K'], ['D','K'], ['M','K'], ['H','K'], ['V','K'],
            ['N','K'], ['-','C'], ['U','C'], ['G','C'], ['K','C'], ['C','C'], ['Y','C'],
            ['S','C'], ['B','C'], ['A','C'], ['W','C'], ['R','C'], ['D','C'], ['M','C'],
            ['H','C'], ['V','C'], ['N','C'], ['-','Y'], ['U','Y'], ['G','Y'], ['K','Y'],
            ['C','Y'], ['Y','Y'], ['S','Y'], ['B','Y'], ['A','Y'], ['W','Y'], ['R','Y'],
            ['D','Y'], ['M','Y'], ['H','Y'], ['V','Y'], ['N','Y'], ['-','S'], ['U','S'],
            ['G','S'], ['K','S'], ['C','S'], ['Y','S'], ['S','S'], ['B','S'], ['A','S'],
            ['W','S'], ['R','S'], ['D','S'], ['M','S'], ['H','S'], ['V','S'], ['N','S'],
            ['-','B'], ['U','B'], ['G','B'], ['K','B'], ['C','B'], ['Y','B'], ['S','B'],
            ['B','B'], ['A','B'], ['W','B'], ['R','B'], ['D','B'], ['M','B'], ['H','B'],
            ['V','B'], ['N','B'], ['-','A'], ['U','A'], ['G','A'], ['K','A'], ['C','A'],
            ['Y','A'], ['S','A'], ['B','A'], ['A','A'], ['W','A'], ['R','A'], ['D','A'],
            ['M','A'], ['H','A'], ['V','A'], ['N','A'], ['-','W'], ['U','W'], ['G','W'],
            ['K','W'], ['C','W'], ['Y','W'], ['S','W'], ['B','W'], ['A','W'], ['W','W'],
            ['R','W'], ['D','W'], ['M','W'], ['H','W'], ['V','W'], ['N','W'], ['-','R'],
            ['U','R'], ['G','R'], ['K','R'], ['C','R'], ['Y','R'], ['S','R'], ['B','R'],
            ['A','R'], ['W','R'], ['R','R'], ['D','R'], ['M','R'], ['H','R'], ['V','R'],
            ['N','R'], ['-','D'], ['U','D'], ['G','D'], ['K','D'], ['C','D'], ['Y','D'],
            ['S','D'], ['B','D'], ['A','D'], ['W','D'], ['R','D'], ['D','D'], ['M','D'],
            ['H','D'], ['V','D'], ['N','D'], ['-','M'], ['U','M'], ['G','M'], ['K','M'],
            ['C','M'], ['Y','M'], ['S','M'], ['B','M'], ['A','M'], ['W','M'], ['R','M'],
            ['D','M'], ['M','M'], ['H','M'], ['V','M'], ['N','M'], ['-','H'], ['U','H'],
            ['G','H'], ['K','H'], ['C','H'], ['Y','H'], ['S','H'], ['B','H'], ['A','H'],
            ['W','H'], ['R','H'], ['D','H'], ['M','H'], ['H','H'], ['V','H'], ['N','H'],
            ['-','V'], ['U','V'], ['G','V'], ['K','V'], ['C','V'], ['Y','V'], ['S','V'],
            ['B','V'], ['A','V'], ['W','V'], ['R','V'], ['D','V'], ['M','V'], ['H','V'],
            ['V','V'], ['N','V'], ['-','N'], ['U','N'], ['G','N'], ['K','N'], ['C','N'],
            ['Y','N'], ['S','N'], ['B','N'], ['A','N'], ['W','N'], ['R','N'], ['D','N'],
            ['M','N'], ['H','N'], ['V','N'], ['N','N']
        ];
        const CHARS_LUT_T: [[char;2];256] = [
            ['-','-'], ['T','-'], ['G','-'], ['K','-'], ['C','-'], ['Y','-'], ['S','-'],
            ['B','-'], ['A','-'], ['W','-'], ['R','-'], ['D','-'], ['M','-'], ['H','-'],
            ['V','-'], ['N','-'], ['-','T'], ['T','T'], ['G','T'], ['K','T'], ['C','T'],
            ['Y','T'], ['S','T'], ['B','T'], ['A','T'], ['W','T'], ['R','T'], ['D','T'],
            ['M','T'], ['H','T'], ['V','T'], ['N','T'], ['-','G'], ['T','G'], ['G','G'],
            ['K','G'], ['C','G'], ['Y','G'], ['S','G'], ['B','G'], ['A','G'], ['W','G'],
            ['R','G'], ['D','G'], ['M','G'], ['H','G'], ['V','G'], ['N','G'], ['-','K'],
            ['T','K'], ['G','K'], ['K','K'], ['C','K'], ['Y','K'], ['S','K'], ['B','K'],
            ['A','K'], ['W','K'], ['R','K'], ['D','K'], ['M','K'], ['H','K'], ['V','K'],
            ['N','K'], ['-','C'], ['T','C'], ['G','C'], ['K','C'], ['C','C'], ['Y','C'],
            ['S','C'], ['B','C'], ['A','C'], ['W','C'], ['R','C'], ['D','C'], ['M','C'],
            ['H','C'], ['V','C'], ['N','C'], ['-','Y'], ['T','Y'], ['G','Y'], ['K','Y'],
            ['C','Y'], ['Y','Y'], ['S','Y'], ['B','Y'], ['A','Y'], ['W','Y'], ['R','Y'],
            ['D','Y'], ['M','Y'], ['H','Y'], ['V','Y'], ['N','Y'], ['-','S'], ['T','S'],
            ['G','S'], ['K','S'], ['C','S'], ['Y','S'], ['S','S'], ['B','S'], ['A','S'],
            ['W','S'], ['R','S'], ['D','S'], ['M','S'], ['H','S'], ['V','S'], ['N','S'],
            ['-','B'], ['T','B'], ['G','B'], ['K','B'], ['C','B'], ['Y','B'], ['S','B'],
            ['B','B'], ['A','B'], ['W','B'], ['R','B'], ['D','B'], ['M','B'], ['H','B'],
            ['V','B'], ['N','B'], ['-','A'], ['T','A'], ['G','A'], ['K','A'], ['C','A'],
            ['Y','A'], ['S','A'], ['B','A'], ['A','A'], ['W','A'], ['R','A'], ['D','A'],
            ['M','A'], ['H','A'], ['V','A'], ['N','A'], ['-','W'], ['T','W'], ['G','W'],
            ['K','W'], ['C','W'], ['Y','W'], ['S','W'], ['B','W'], ['A','W'], ['W','W'],
            ['R','W'], ['D','W'], ['M','W'], ['H','W'], ['V','W'], ['N','W'], ['-','R'],
            ['T','R'], ['G','R'], ['K','R'], ['C','R'], ['Y','R'], ['S','R'], ['B','R'],
            ['A','R'], ['W','R'], ['R','R'], ['D','R'], ['M','R'], ['H','R'], ['V','R'],
            ['N','R'], ['-','D'], ['T','D'], ['G','D'], ['K','D'], ['C','D'], ['Y','D'],
            ['S','D'], ['B','D'], ['A','D'], ['W','D'], ['R','D'], ['D','D'], ['M','D'],
            ['H','D'], ['V','D'], ['N','D'], ['-','M'], ['T','M'], ['G','M'], ['K','M'],
            ['C','M'], ['Y','M'], ['S','M'], ['B','M'], ['A','M'], ['W','M'], ['R','M'],
            ['D','M'], ['M','M'], ['H','M'], ['V','M'], ['N','M'], ['-','H'], ['T','H'],
            ['G','H'], ['K','H'], ['C','H'], ['Y','H'], ['S','H'], ['B','H'], ['A','H'],
            ['W','H'], ['R','H'], ['D','H'], ['M','H'], ['H','H'], ['V','H'], ['N','H'],
            ['-','V'], ['T','V'], ['G','V'], ['K','V'], ['C','V'], ['Y','V'], ['S','V'],
            ['B','V'], ['A','V'], ['W','V'], ['R','V'], ['D','V'], ['M','V'], ['H','V'],
            ['V','V'], ['N','V'], ['-','N'], ['T','N'], ['G','N'], ['K','N'], ['C','N'],
            ['Y','N'], ['S','N'], ['B','N'], ['A','N'], ['W','N'], ['R','N'], ['D','N'],
            ['M','N'], ['H','N'], ['V','N'], ['N','N']
        ];
        match T {
            'T' => CHARS_LUT_T[usize::from(c)],
            'U' => CHARS_LUT_U[usize::from(c)],
            _ => unreachable!()
        }
    }

    #[cfg(all(target_arch="x86_64",feature="simd"))]
    #[inline]
    fn decode_simd_x86<const T: char>(inbuf: *const u8, output_lo: *mut [u8;16], output_hi: *mut [u8;16]) -> Result<(),std::io::Error>{
        unsafe {
            // Lookup Vector 
            let lookup_vec: __m128i = _mm_set_epi8( // SSE2
                'N' as i8,
                'V' as i8,
                'H' as i8,
                'M' as i8,
                'D' as i8,
                'R' as i8,
                'W' as i8,
                'A' as i8,
                'B' as i8,
                'S' as i8,
                'Y' as i8,
                'C' as i8,
                'K' as i8,
                'G' as i8,
                 T as i8,
                '-' as i8,
                );
            let shiftval = _mm_set1_epi32(4); // SSE2
            // 4bit mask for each byte
            let lo_byte_vec: __m128i = _mm_set1_epi8(0x0f); // SSE2
            // encoded input buffer
            let mut mmvec = _mm_loadu_si128(inbuf.cast()); // SSE2
            // shuffle only uses the bottom 4 bits (AND masked), get the low byte
            let lobyte = _mm_shuffle_epi8(lookup_vec, _mm_and_si128(mmvec,lo_byte_vec)); // SSSE3
            // shift 4 bits to the right
            mmvec = _mm_srlv_epi32(mmvec, shiftval); // AVX2
            // perform the shuffle again
            let hibyte = _mm_shuffle_epi8(lookup_vec, _mm_and_si128(mmvec,lo_byte_vec)); // SSSE3
            // Unpack the low 8 bytes (interleaved) 
            let outvec2 = _mm_unpacklo_epi8(lobyte,hibyte); // SSE2
            // unpack the high 8 bytes (interleaved)
            let outvec1 = _mm_unpackhi_epi8(lobyte,hibyte); // SSE2
            // store the vectors to the output arrays
            _mm_storeu_si128(output_hi.cast(),outvec1); // SSE2
            _mm_storeu_si128(output_lo.cast(),outvec2); // SSE2
        }
        Ok(())
    }

    #[cfg(feature="simd")]
    #[inline]
    fn decode_simd<const T:char>(inbuf: [u8;16]) -> Result<[u8;32],std::io::Error> {
        #[cfg(not(target_arch="x86_64"))] // Add architectures here if adding
        return Err(SomeError); 
        // guarantee that CPU has SSE2, SSSE3, and AVX2 
        #[cfg(target_arch="x86_64")]
        {
            unsafe {
                let feature_cpuid = __cpuid_count(1,0);
                if __cpuid_count(7,0).ebx & (1<<5) == 0 || // AVX2
                   feature_cpuid.edx & (1<<26) == 0 ||      // SSE2
                   feature_cpuid.ecx & (1<<9) == 0          // SSSE3
                {
                    // this should result in a warning and failover from the calling function
                    return Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"Not supported by CPU"));
                }
            }
            let mut output_lo: [u8;16] = [0;16];
            let mut output_hi: [u8;16] = [0;16];
            let inbuf_slice = inbuf.as_ptr();
            Self::decode_simd_x86::<T>(inbuf_slice,&mut output_lo,&mut output_hi)?; // caller fails
            // on error
            let mut outbuf = [0;32];
            for i in 0..32 {
                outbuf[i]  = if i<16 { output_lo[i] } else { output_hi[i-16] }
            }
            Ok(outbuf)
        }
    }

    #[inline]
    fn decode<const T: char>(c: u8) -> char {
        match c {
            0x00 => '-',
            0x01 => T,
            0x02 => 'G',
            0x03 => 'K',
            0x04 => 'C',
            0x05 => 'Y',
            0x06 => 'S',
            0x07 => 'B',
            0x08 => 'A',
            0x09 => 'W',
            0x0A => 'R',
            0x0B => 'D',
            0x0C => 'M',
            0x0D => 'H',
            0x0E => 'V',
            0x0F => 'N',
            _ => unreachable!(),
        }
    }
}

// --- MaskReader --------------------------------------------------------------

#[derive(Debug)]
pub struct MaskReader<R: BufRead> {
    reader: R,
    total: u64,
    current: u64,
    mask: bool,
}

impl<R: BufRead> MaskReader<R> {
    pub fn new(reader: R, total: u64) -> Self {
        Self {
            reader,
            total,
            current: 0,
            mask: false,
        }
    }
}

impl<R: BufRead> Iterator for MaskReader<R> {
    type Item = Result<MaskUnit, std::io::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }

        let mut n = 0u64;
        loop {
            let mut i = 0;
            let buf = match self.reader.fill_buf() {
                Err(e) => return Some(Err(e)),
                Ok([]) => break,
                Ok(buf) => buf,
            };
            while i < buf.len() && buf[i] == 0xFF {
                n += 0xFF;
                i += 1;
            }
            if i < buf.len() {
                n += buf[i] as u64;
                self.reader.consume(i + 1);
                break;
            }
            self.reader.consume(i);
        }

        self.current += n;
        if self.mask {
            self.mask = false;
            Some(Ok(MaskUnit::Masked(n)))
        } else {
            self.mask = true;
            Some(Ok(MaskUnit::Unmasked(n)))
        }
    }
}
