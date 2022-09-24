use crate::bitstream::BitReader;

#[derive(Eq, PartialEq, Default, Copy, Clone)]
pub(crate) struct HuffmanCode {
    pub code: u16,
    // 0 if invalid
    pub bits: u8,
}

pub(crate) struct HuffmanTree {
    pub lookup: [(HuffmanCode, u8); 1 << 16],
}

pub fn to_index(code: u16, bits: u32) -> usize {
    code.rotate_right(bits) as usize
}

impl HuffmanTree {
    pub fn new() -> Self {
        Self {
            lookup: [(HuffmanCode { code: 0, bits: 0 }, 0); 1 << 16],
        }
    }

    #[inline(never)]
    pub fn read_code(&self, bitreader: &mut BitReader) -> Option<u8> {
        let mut hc: HuffmanCode = Default::default();
        for _ in 0..16 {
            let bit1 = bitreader.get_bit();

            let bit = bit1?;

            hc.bits += 1;
            hc.code <<= 1;
            hc.code |= bit as u16;

            let index = to_index(hc.code, hc.bits as u32);

            let (vcode, symbol) = self.lookup[index];

            if vcode == hc {
                return Some(symbol);
            }
        }

        None
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
