use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::mem::size_of;

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
        for &x in chunk {
            print!("{x: >3} ");
        }
        println!();
    }
}

fn print_dst_quant_table(dst: u8) {
    match dst {
        0 => println!("0 - luminance"),
        1 => println!("1 - chrominance"),
        _ => unreachable!("invalid dst for quant matrix"),
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

                // TODO we handle this incorrectly for 16-bit
                assert!(len == 64);

                let qt_info = read_u8(&mut reader)?;

                // bottom 4 bits are the actual dst
                let dst = qt_info & 0xf;

                // if upper 4 bits are 0, 8-bit
                // otherwise 16-bit
                let qt_is_8_bit = (qt_info & 0xf0) == 0;

                let mut quant_table = [0; 8 * 8];

                // TODO this isn't correct for 16-bit
                reader.read_exact(&mut quant_table)?;

                println!("Quant Matrix: {}-bit", if qt_is_8_bit { "8" } else { "16" });
                print_dst_quant_table(dst);
                print_8x8_quant_table(&quant_table);
                println!();
            }
            JPEG_DEFINE_HUFFMAN_TABLE => {
                // Not actually needed, but we do have to advance forward 2 bytes.
                let _len = read_u16(&mut reader)?;

                let ht_info = read_u8(&mut reader)?;

                let ht_num = ht_info & 0xf;
                assert!(ht_num <= 3);

                // bit index 4 (5th bit) specifies whether table is for AC/DC
                // 0 = DC, 1 = AC
                let ht_is_dc = ((ht_info & (1 << 4)) >> 4) == 0;

                // TODO maybe make a build flag for extra checks or something
                // ensure bit index 5-7 is 0
                assert!(ht_info & 0b1110_0000 == 0);

                println!(
                    "Component {ht_num}, {} huffman tree",
                    if ht_is_dc { "DC" } else { "AC" }
                );

                // read 16 bytes for child node counts for 16 levels of huffman tree
                let mut buf = [0; 16];

                reader.read_exact(&mut buf)?;
                println!("buf: {buf:?}");

                let mut code = 0u16;
                let mut bits = 0;

                // println!("[Symbol] [Code]:");

                for tdepth in buf {
                    code <<= 1;
                    bits += 1;

                    // TODO optimize symbol decoding
                    for _ in 0..tdepth {
                        let symbol = read_u8(&mut reader)?;

                        // println!("{symbol: >3}  :  {:0width$b}", code, width = bits);

                        code += 1;
                    }
                }

                println!("Elements: {}", buf.iter().map(|x| *x as u32).sum::<u32>());

                // TODO for check_decoder, ensure symbols read equals
                // sum of symbols read, and complies with the length
            }
            // Other currently unsupported marker
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
