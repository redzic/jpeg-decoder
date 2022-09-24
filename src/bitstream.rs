use std::fs::File;
use std::io;
use std::io::{BufReader, Read};

/// Reads unsigned short in big-endian format
pub fn read_u16(reader: &mut BufReader<File>) -> io::Result<u16> {
    let mut buf = [0; 2];
    reader.read_exact(&mut buf)?;

    Ok(u16::from_be_bytes(buf))
}

/// Reads byte
pub fn read_u8(reader: &mut BufReader<File>) -> io::Result<u8> {
    let mut buf = [0];
    reader.read_exact(&mut buf)?;

    Ok(buf[0])
}

pub(crate) struct BitReader<'a> {
    reader: &'a mut BufReader<File>,
    // cached bits
    bitbuf: u64,
    bitlen: u32,
}

impl<'a> BitReader<'a> {
    pub fn new(reader: &'a mut BufReader<File>) -> Self {
        Self {
            reader,
            bitbuf: 0,
            bitlen: 0,
        }
    }

    fn byte_refill(&mut self) -> Option<u8> {
        // skip over 0x00 in 0xff00 found in bitstream
        let new_byte = read_u8(self.reader).ok()?;

        if new_byte == 0xff {
            let next_byte = read_u8(self.reader).ok()?;

            if next_byte != 0x00 {
                return None;
            }
        }

        Some(new_byte)
    }

    // Only use for reading start of scan data
    // Not a general purpose get_bit function
    pub fn get_bit(&mut self) -> Option<bool> {
        // refill buffer
        if self.bitlen == 0 {
            let new_byte = self.byte_refill()?;

            self.bitbuf |= (new_byte as u64).rotate_right(8);

            self.bitlen = 8;
        }

        self.bitlen -= 1;
        let bit = self.bitbuf >> 63 != 0;
        self.bitbuf <<= 1;
        Some(bit)
    }

    #[inline(never)]
    pub fn get_n_bits(&mut self, bits: u32) -> Option<u16> {
        assert!(bits <= 16);

        while self.bitlen < bits {
            let byte = self.byte_refill()?;
            self.bitbuf |= (byte as u64).rotate_right(8) >> self.bitlen;
            self.bitlen += 8;
        }

        let code = (self.bitbuf >> (64 - bits)) as u16;
        self.bitbuf <<= bits;
        self.bitlen -= bits;
        return Some(code);
    }
}
