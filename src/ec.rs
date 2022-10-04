use crate::bitstream::BitReader;

pub(crate) struct HuffmanTree {
    pub symbols: Box<[u8]>,
    pub cht: Box<[(u16, u8, u8)]>,
    // min code length
    pub l0: u8,
}

impl HuffmanTree {
    pub fn new() -> Self {
        Self {
            cht: Box::new([]),
            symbols: Box::new([]),
            l0: 0,
        }
    }

    #[inline(never)]
    pub fn read_code(&self, bitreader: &mut BitReader) -> Option<u8> {
        let mut w = bitreader.peek_bits::<16>()?;

        if w < self.cht[0].0 {
            w >>= 16 - self.l0;

            bitreader.consume_bits(self.l0 as u32);

            Some(self.symbols[w as usize])
        } else {
            // TODO rewrite as functional
            let mut j = None;
            for i in 1..self.cht.len() {
                if self.cht[i].0 > w {
                    j = Some(i - 1);
                    break;
                }
            }
            let j = j.unwrap_or(self.cht.len() - 1);

            // aug_c = augmented codeword
            let (aug_c, l, offset) = self.cht[j];

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
