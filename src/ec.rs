use std::arch::x86_64::*;

use crate::bitstream::BitReader;

pub(crate) struct HuffmanTree {
    pub symbols: Box<[u8]>,
    // pub cht: Box<[(u16, u8, u8)]>,

    // TODO it would be better if these were all packed but whatever

    // condensed huffman tree fields
    pub aug: Box<[u16]>,
    pub l: Box<[u8]>,
    pub offset: Box<[u8]>,

    // min code length
    pub l0: u8,
}

impl HuffmanTree {
    pub fn new() -> Self {
        Self {
            aug: Box::new([]),
            l: Box::new([]),
            offset: Box::new([]),

            symbols: Box::new([]),
            l0: 0,
        }
    }

    pub fn read_code(&self, bitreader: &mut BitReader) -> Option<u8> {
        let mut w = bitreader.peek_bits::<16>()?;

        if w < self.aug[0] {
            w >>= 16 - self.l0;

            bitreader.consume_bits(self.l0 as u32);

            Some(self.symbols[w as usize])
        } else {
            // TODO rewrite as functional
            // let mut j = None;

            // for i in 1..self.aug.len() {
            //     if self.aug[i] > w {
            //         j = Some(i - 1);
            //         break;
            //     }
            // }
            // let j = j.unwrap_or(self.aug.len() - 1);

            let j = unsafe {
                let v2 = _mm256_set1_epi16(1 << 15);

                let v = _mm256_loadu_si256(self.aug.as_ptr().add(1).cast());

                let wv = _mm256_set1_epi16(w as i16);
                let wv = _mm256_xor_si256(wv, v2);

                let v = _mm256_xor_si256(v, v2);

                let cmp = _mm256_cmpgt_epi16(v, wv);
                let mask = _mm256_movemask_epi8(cmp);

                usize::min((mask.trailing_zeros() / 2) as usize, self.aug.len() - 1)
            };

            let (aug_c, l, offset) = (self.aug[j], self.l[j], self.offset[j]);

            w >>= 16 - l;

            bitreader.consume_bits(l as u32);

            let base = aug_c >> (16 - l);
            Some(self.symbols[w as usize - base as usize + offset as usize])
        }
    }
}

pub fn sign_code(n_bits: u32, code: u16) -> i16 {
    if ((code as u32) << 1) >> n_bits != 0 {
        code as i16
    } else {
        let max_val = (1 << n_bits) - 1;
        code as i16 - max_val
    }
}
