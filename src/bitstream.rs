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

    cached_byte: Option<u8>,

    bit_offset: u32,
}

impl<'a> BitReader<'a> {
    pub fn new(reader: &'a mut BufReader<File>) -> Self {
        Self {
            reader,
            bit_offset: 0,
            cached_byte: None,
        }
    }

    // Only use for reading start of scan data
    // Not a general purpose get_bit function
    pub fn get_bit(&mut self) -> Option<bool> {
        // skip over 0x00 in 0xff00 found in bitstream

        // TODO this could be optimized to use shift by 1
        // instead of dynamic shift
        loop {
            if let Some(byte) = self.cached_byte {
                let shift = 7 - self.bit_offset;

                let bit = (byte >> shift) & 1 != 0;

                self.bit_offset = (self.bit_offset + 1) % 8;

                if self.bit_offset == 0 {
                    // reached end of byte, read next byte
                    let cached_byte = read_u8(&mut self.reader).ok()?;

                    if cached_byte == 0xff {
                        let next_byte = read_u8(&mut self.reader).ok()?;
                        if next_byte != 0x00 {
                            return None;
                        }
                    }

                    self.cached_byte = Some(cached_byte);
                }

                return Some(bit);
            } else {
                let byte = read_u8(&mut self.reader).ok()?;
                self.cached_byte = Some(byte);

                continue;
            }
        }
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
