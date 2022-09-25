use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::mem::transmute;

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

pub struct InnerBuf<'a> {
    buf: Vec<u8>,
    reader: &'a mut BufReader<File>,
}

impl<'a> InnerBuf<'a> {
    fn new(reader: &'a mut BufReader<File>) -> Self {
        Self {
            buf: Vec::new(),
            reader,
        }
    }

    // SAFETY: caller should not access the contents of `slice` if
    // InnerBuf has been dropped.
    unsafe fn refill(&mut self, slice: &mut &[u8]) {
        self.buf.clear();
        let n = self.reader.read_until(0xff, &mut self.buf).unwrap();

        if n == 0 {
            // no data left
            *slice = &[];
        } else {
            let next_byte = read_u8(self.reader).unwrap();

            if next_byte != 0x00 {
                *slice = &[];
                return;
            }

            // TODO figure out safe alternative to this
            *slice = transmute(self.buf.as_slice());
        }
    }
}

pub(crate) struct BitReader<'a> {
    stream: &'a [u8],

    buf: InnerBuf<'a>,

    // cached bits
    bitbuf: u64,
    bitlen: u32,
}

impl<'a> BitReader<'a> {
    pub fn new(x: &'a mut BufReader<File>) -> Self {
        Self {
            stream: &[],
            bitbuf: 0,
            bitlen: 0,
            buf: InnerBuf::new(x),
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        // refill buffer
        if self.stream.is_empty() {
            unsafe {
                self.buf.refill(&mut self.stream);
            }

            // No more data left
            if self.stream.is_empty() {
                return None;
            }
        }

        let byte = self.stream[0];

        self.stream = &self.stream[1..];

        Some(byte)
    }

    // Only use for reading start of scan data
    // Not a general purpose get_bit function
    #[allow(unused)]
    pub fn get_bit(&mut self) -> Option<bool> {
        // refill buffer
        if self.bitlen == 0 {
            // TODO is there a subtle bug here?
            // like we should refill at least once and return
            // an error if the first refill failed
            while let Some(byte) = self.read_byte() {
                self.bitbuf |= (byte as u64).rotate_right(8 + self.bitlen);
                self.bitlen += 8;
                if self.bitlen == 64 - 16 {
                    break;
                }
            }
        }

        self.bitlen -= 1;
        let bit = self.bitbuf >> 63 != 0;
        self.bitbuf <<= 1;
        Some(bit)
    }

    pub fn peek_bits<const BITS: u32>(&mut self) -> Option<u16> {
        assert!(BITS <= 16);

        while self.bitlen < BITS {
            // pad with zeroes if nothing is left
            let byte = match self.byte_refill() {
                Some(x) => x,
                None => break,
            };
            self.bitbuf |= (byte as u64).rotate_right(8) >> self.bitlen;
            self.bitlen += 8;
        }

        let code = (self.bitbuf >> (64 - BITS)) as u16;

        Some(code)
    }

    pub fn consume_bits(&mut self, bits: u32) {
        self.bitbuf <<= bits;
        self.bitlen -= bits;
    }

    pub fn get_n_bits(&mut self, bits: u32) -> Option<u16> {
        debug_assert!(bits <= 16);

        // TODO maybe refill to max size here as well
        while self.bitlen < bits {
            let byte = self.read_byte()?;
            self.bitbuf |= (byte as u64).rotate_right(8) >> self.bitlen;
            self.bitlen += 8;
        }

        let code = self.bitbuf.rotate_left(bits) as u16;
        self.bitbuf <<= bits;
        self.bitlen -= bits;
        Some(code)
    }
}
