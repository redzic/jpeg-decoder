use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
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

const JPEG_START_OF_IMAGE: u16 = 0xffd8;
const JPEG_APPLICATION_DEFAULT_HEADER: u16 = 0xffe0;
const JPEG_QUANTIZATION_TABLE: u16 = 0xffdb;
const JPEG_START_OF_FRAME: u16 = 0xffc0;
const JPEG_DEFINE_HUFFMAN_TABLE: u16 = 0xffc4;
const JPEG_START_OF_SCAN: u16 = 0xffda;
const JPEG_END_OF_IMAGE: u16 = 0xffd9;

fn get_jpeg_segment_name(marker: u16) -> &'static str {
    match marker {
        JPEG_START_OF_IMAGE => "Start of Image",
        JPEG_APPLICATION_DEFAULT_HEADER => "Application Default Header",
        JPEG_QUANTIZATION_TABLE => "Quantization Table",
        JPEG_START_OF_FRAME => "Start of Frame",
        JPEG_DEFINE_HUFFMAN_TABLE => "Define Huffman Table",
        JPEG_START_OF_SCAN => "Start of Scan",
        JPEG_END_OF_IMAGE => "End of Image",
        _ => panic!("invalid jpeg marker"),
    }
}

/// Reads unsigned short in big-endian format
fn read_u16(reader: &mut BufReader<File>) -> io::Result<u16> {
    let mut buf = [0; 2];
    reader.read_exact(&mut buf)?;

    Ok(u16::from_be_bytes(buf))
}

/// Reads byte
fn read_u8(reader: &mut BufReader<File>) -> io::Result<u8> {
    let mut buf = [0];
    reader.read_exact(&mut buf)?;

    Ok(buf[0])
}

fn print_8x8_quant_table(x: &[u8; 64]) {
    for chunk in x.chunks_exact(8) {
        println!("{chunk:?}");
    }
}

fn print_dst_quant_table(dst: u8) {
    match dst {
        0 => println!("0 - luminance"),
        1 => println!("1 - chrominance"),
        _ => unreachable!("invalid dst"),
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
            JPEG_START_OF_IMAGE => {}
            JPEG_END_OF_IMAGE => {}
            // Start of scan (actual entropy coded image data)
            JPEG_START_OF_SCAN => {
                // Don't process for now, just skip to the end,
                // which should contain 0xffd9 to indicate the
                // end of the image.
                reader.seek(SeekFrom::End(-2))?;
            }
            JPEG_APPLICATION_DEFAULT_HEADER => {
                let len = read_u16(&mut reader)?;

                let mut null_str = Vec::new();

                // TODO read len-2 bytes upfront, and search that area instead
                // of doing it this pretty garbage way

                // read null-terminated string
                let n_read = reader.read_until(0, &mut null_str)?;
                assert!(
                    n_read <= len as usize - size_of::<u16>(),
                    "Invalid length after marker in Application Default Header"
                );
                // TODO technically not invalid length, but actually the string info
                // or whatever is just too long

                // TODO make read<N> helper function
                let v_maj = read_u8(&mut reader)?;
                let v_min = read_u8(&mut reader)?;

                let units = read_u8(&mut reader)?;

                let dx = read_u16(&mut reader)?;
                let dy = read_u16(&mut reader)?;

                // Thumbnail information
                let tx = read_u8(&mut reader)?;
                let ty = read_u8(&mut reader)?;

                let s = std::str::from_utf8(&null_str[..null_str.len() - 1])
                    .expect("Invalid UTF-8 in Application Default Header identifier");

                println!();
                println!("Identifier:   {s}");
                println!("Version:      {v_maj}.{v_min}");
                println!("Units:        {units} (dpi)");
                println!("Density:      {dx}x{dy}");
                println!("Thumbnail:    {tx}x{ty}\n");
            }
            JPEG_QUANTIZATION_TABLE => {
                let len = read_u16(&mut reader)? as usize - 3;

                // Umm.. isn't the length just always 64, when you subtract the destination byte?
                assert!(len == 64);

                let dst = read_u8(&mut reader)?;

                let mut quant_table = [0; 8 * 8];

                reader.read_exact(&mut quant_table)?;

                print_dst_quant_table(dst);
                print_8x8_quant_table(&quant_table);
                println!();
            }
            _ => {
                // read another BE u16, which indicates the length
                let len = read_u16(&mut reader)?;

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
