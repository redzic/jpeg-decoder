use crate::bitstream::BitReader;

#[derive(Eq, PartialEq, Default, Copy, Clone)]
pub(crate) struct HuffmanCode {
    pub code: u16,
    pub bits: u16,
}

pub(crate) struct HuffmanTree {
    pub lookup: [Option<(u8, HuffmanCode)>; 65_536],
}

pub fn to_index(code: u16, bits: u32) -> usize {
    // TODO investigate alternative tradeoffs between likelihood
    // of false positive and overhead of computing to_index

    // for example:
    // let mask = u16::MAX >> bits;

    // (code.rotate_right(bits) | mask) as usize

    // however, honestly I think for larger speedups we'd have to rework
    // our approach to huffman decoding anyway. so don't get too caught
    // up in this.

    code.rotate_right(bits) as usize
}

impl HuffmanTree {
    pub fn new() -> Self {
        Self {
            lookup: [None; 65_536],
        }
    }

    pub fn read_code(&self, bitreader: &mut BitReader) -> Option<u8> {
        let mut code: HuffmanCode = Default::default();
        loop {
            let bit1 = bitreader.get_bit();

            let bit = bit1?;

            code.bits += 1;
            code.code <<= 1;
            code.code |= bit as u16;

            let lookup = to_index(code.code, code.bits as u32);

            if let Some((symbol, code2)) = self.lookup[lookup] {
                if code == code2 {
                    return Some(symbol);
                }
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
