pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    bit_idx: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, bit_idx: 0 }
    }

    pub fn get_bit(&mut self) -> Option<bool> {
        let byte_idx = self.bit_idx / 8;

        if let Some(&x) = self.data.get(byte_idx) {
            let bit_offset = 7 - self.bit_idx % 8;
            let ret = (x >> bit_offset) & 1 != 0;

            // for next iteration
            self.bit_idx += 1;

            return Some(ret);
        }

        None
    }

    pub fn get_n_bits(&mut self, bits: u32) -> Option<u16> {
        assert!(bits <= 16);

        let mut code = 0;

        for _ in 0..bits {
            let bit = self.get_bit()? as u16;
            code <<= 1;
            code |= bit;
        }

        Some(code)
    }
}
