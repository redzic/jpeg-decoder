use std::collections::HashMap;

use crate::bitstream::BitReader;

#[derive(Hash, Eq, PartialEq)]
pub(crate) struct HuffmanCode {
    pub code: u16,
    pub bits: u8,
}

impl Default for HuffmanCode {
    fn default() -> Self {
        Self { code: 0, bits: 0 }
    }
}

pub(crate) struct HuffmanTree {
    pub lookup: HashMap<HuffmanCode, u8>,
}

impl HuffmanTree {
    pub fn new() -> Self {
        Self {
            lookup: HashMap::new(),
        }
    }

    pub fn read_code(&self, bitreader: &mut BitReader) -> Option<u8> {
        let mut code: HuffmanCode = Default::default();
        loop {
            let bit = bitreader.get_bit()?;

            code.bits += 1;
            code.code <<= 1;
            code.code |= bit as u16;

            if let Some(&symbol) = self.lookup.get(&code) {
                return Some(symbol);
            }
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
