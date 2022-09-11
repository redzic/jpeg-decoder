use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Read, Seek, SeekFrom},
};

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
        _ => unreachable!(),
    }
}

fn main() -> Result<(), std::io::Error> {
    // TODO incrementally read the file
    // let bytes = fs::read("./out.jpg").unwrap();

    // let mut reader = BufReader::new(File::open("./out.jpg")?);
    let mut reader = BufReader::new(File::open("./profile.jpg")?);

    // let header = slice::<_, 2>(&bytes);

    let mut buf = [0; 2];

    loop {
        // read >H (python unpack notation), this means
        // we read a big-endian unsigned short.

        // reader.read_exact(&mut buf)?;
        if let Err(_) = reader.read_exact(&mut buf) {
            break;
        }

        let marker = u16::from_be_bytes(buf);

        println!("{}", get_jpeg_segment_name(marker));

        match marker {
            // start of sequence
            0xffd8 => {}
            0xffd9 => {}
            // Start of scan
            0xffda => {
                // skip to the end I guess?
                reader.seek(SeekFrom::End(-2))?;
            }
            _ => {
                // read another BE u16, which indicates the length
                reader.read_exact(&mut buf)?;
                let len = u16::from_be_bytes(buf);

                // The readed length includes the size of itself,
                // but since we advanced the reader 2 bytes to actually
                // read the length, we need to subtract by 2 to seek
                // by the correct amount.
                reader.seek(SeekFrom::Current(len as i64 - 2))?;
            }
        }
    }

    Ok(())
}
