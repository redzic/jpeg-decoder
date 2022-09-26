use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::mem::transmute;

use crate::util::{likely, unlikely};

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

// TODO write tests for this
pub struct InnerBuf<'a> {
    reader: &'a mut BufReader<File>,
    bpos: usize,
}

impl<'a> InnerBuf<'a> {
    fn new(reader: &'a mut BufReader<File>) -> Self {
        Self {
            reader,
            bpos: 0,
            // eob_0xff: false,
        }
    }

    // SAFETY: caller should not access the contents of `slice` if
    // InnerBuf has been dropped.

    // TODO prefer returning slice by value, since we don't want this function
    // to be inlined, and returning by value (in registers) is more efficient.
    #[inline(never)]
    unsafe fn refill(&mut self, slice: &mut &[u8]) {
        // need to actually refill buffer, not just progress through
        // current buffer
        if unlikely(self.bpos >= self.reader.buffer().len()) {
            // consume the whole buffer
            self.reader.consume(self.reader.buffer().len());
            self.bpos = 0;

            match self.reader.fill_buf() {
                Ok(_new_buf) => {
                    // continue as normal
                }
                _ => {
                    // shouldn't happen?
                    unreachable!();
                }
            }
        }

        // find next 0xff in current buffer, if any
        let pos_0xff = memchr::memchr(0xff, &self.reader.buffer()[self.bpos..]);

        // after first 0xff, everything is fucked up

        if let Some(pos) = pos_0xff {
            // return bytes up to and including the 0xff found, only
            // if the byte after the 0xff is 0
            if let Some(&next_byte) = self.reader.buffer().get(self.bpos + pos + 1) {
                if likely(next_byte == 0x00) {
                    // all is good, we just have to send bytes not including the 0x00

                    // to include the 0xff we found:
                    // for example if pos==1, we have to go up to and including index 1.

                    // pos is index of 0xff relative to bpos

                    // pos is relative to self.bpos
                    *slice = transmute(&self.reader.buffer()[self.bpos..][..=pos]);

                    // we found 0xff00 at index bpos+pos,
                    // this means we need to skip the 0 afterwards.
                    // if bpos = 0, pos = 0
                    // we found 0xff at index 0
                    // index 1 = 0x00 (which we skip)
                    // so bpos should be 2 afterwards
                    self.bpos += pos + 2;

                    // dbg!(&self.reader.buffer()[self.bpos - 5..][..5]);
                    // dbg!(&self.reader.buffer()[self.bpos..][..5]);

                    return;
                } else {
                    // Why does it think this is not 0 though?

                    // send bytes not even including the 0xff, since we are at eof now

                    *slice = transmute(&self.reader.buffer()[self.bpos..][..pos]);

                    // we found 0xffd9 (or something)
                    // pos indicates position of 0xffd9

                    // dbg!(next_byte);
                    // dbg!(&self.reader.buffer()[self.bpos - 5..][..5]);
                    // dbg!(&self.reader.buffer()[self.bpos..][..5]);

                    return;
                }
            } else {
                // should only happen when 0xff was found at the end of the buffer
                assert!(pos == self.reader.buffer().len() - 1);

                // handle in next iteration of refill, check first byte

                // TODO handle properly later
                // probably won't happen in practice
                unreachable!();
            }
        } else {
            // assert!(!self.reader.buffer()[self.bpos..].contains(&0xff));

            // send the entire buffer
            *slice = transmute(&self.reader.buffer()[self.bpos..]);
            self.bpos = self.reader.buffer().len();
            return;
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

impl<'a> Drop for InnerBuf<'a> {
    fn drop(&mut self) {
        // consume rest of the buffer
        self.reader.consume(self.reader.buffer().len());
    }
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

    #[inline(always)]
    fn read_byte(&mut self) -> Option<u8> {
        // refill buffer
        if unlikely(self.stream.is_empty()) {
            unsafe {
                self.buf.refill(&mut self.stream);
            }

            // No more data left
            if self.stream.is_empty() {
                return None;
            }
        }

        // unfortunately the compiler can't see that by this point
        // self.stream will never be empty
        if self.stream.is_empty() {
            unsafe {
                std::hint::unreachable_unchecked();
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
                self.bitbuf |= (byte as u64).rotate_right(8) >> self.bitlen;
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
