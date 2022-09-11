use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::mem::size_of;

const JPEG_MARKER: [u8; 2] = [0xff, 0xd8];

fn is_jpeg_marker(header: &[u8; 2]) -> bool {
    *header == JPEG_MARKER
}

#[inline(always)]
fn slice<T, const N: usize>(x: &[T]) -> &[T; N] {
    // SAFETY: if this bounds check succeeds, then
    // it is safe to cast to a fixed-size slice
    // of size N.
    unsafe { &*(&x[..N] as *const [T] as *const [T; N]) }
}

struct JpegDecoder {}

fn get_jpeg_segment_name(marker: u16) -> &'static str {
    match marker {
        0xffd8 => "Start of Image",
        0xffe0 => "Application Default Header",
        0xffdb => "Quantization Table",
        0xffc0 => "Start of Frame",
        0xffc4 => "Define Huffman Table",
        0xffda => "Start of Scan",
        0xffd9 => "End of Image",
        _ => panic!("invalid jpeg marker"),
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut reader = BufReader::new(File::open("./profile.jpg")?);

    let mut buf = [0; 2];

    loop {
        // read >H (python unpack notation), this means
        // we read a big-endian unsigned short.

        if reader.read_exact(&mut buf).is_err() {
            break;
        }

        let marker = u16::from_be_bytes(buf);

        println!("{}", get_jpeg_segment_name(marker));

        match marker {
            // start of sequence
            0xffd8 => {}
            0xffd9 => {}
            // Start of scan (actual entropy coded image data)
            0xffda => {
                // Don't process for now, just skip to the end,
                // which should contain 0xffd9 to indicate the
                // end of the image.
                reader.seek(SeekFrom::End(-(size_of::<u16>() as i64)))?;
            }
            _ => {
                // read another BE u16, which indicates the length
                reader.read_exact(&mut buf)?;
                let len = u16::from_be_bytes(buf);

                // The readed length includes the size of itself,
                // but since we advanced the reader 2 bytes to actually
                // read the length, we need to subtract by 2 to seek
                // by the correct amount.
                reader.seek(SeekFrom::Current(len as i64 - size_of::<u16>() as i64))?;
            }
        }
    }

    Ok(())
}
