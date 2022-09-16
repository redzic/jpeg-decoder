use std::collections::HashMap;
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
        JPEG_QUANTIZATION_TABLE => "Define Quantization Table",
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
        0 => println!("Luminance"),
        1 => println!("Chrominance"),
        _ => unreachable!("invalid dst for quant matrix"),
    }
}

#[derive(Hash, Eq, PartialEq)]
struct HuffmanCode {
    code: u16,
    bits: u8,
}

impl Default for HuffmanCode {
    fn default() -> Self {
        Self { code: 0, bits: 0 }
    }
}

struct HuffmanTree {
    lookup: HashMap<HuffmanCode, u8>,
}

impl HuffmanTree {
    fn new() -> Self {
        Self {
            lookup: HashMap::new(),
        }
    }

    fn read_code(&self, bitreader: &mut BitReader) -> u8 {
        let mut code: HuffmanCode = Default::default();
        loop {
            // read bit
            let bit = bitreader.get_bit().unwrap();

            code.bits += 1;
            code.code <<= 1;
            code.code |= bit as u16;

            if let Some(x) = self.lookup.get(&code) {
                return *x;
            }
        }
    }
}

pub fn sign_code(n_bits: u32, code: u16) -> i16 {
    if ((code as u32) << 1) >> n_bits != 0 {
        code as i16
    } else {
        // 0
        let max_val = (1 << n_bits) - 1;
        code as i16 - max_val
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    bit_idx: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, bit_idx: 0 }
    }

    fn get_bit(&mut self) -> Option<bool> {
        let byte_idx = self.bit_idx / 8;

        if let Some(&x) = self.data.get(byte_idx) {
            let bit_offset = 7 - self.bit_idx % 8;
            let ret = (x >> bit_offset) & 1 != 0;

            // for next iteration
            self.bit_idx += 1;

            return Some(ret);
        }

        // None
        None
    }

    fn get_n_bits(&mut self, bits: u32) -> Option<u16> {
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

fn main() -> Result<(), std::io::Error> {
    let mut reader = BufReader::new(File::open("./profile.jpg")?);

    let mut buf = [0; 2];

    let mut quant_matrices = [[0; 64]; 2];
    let mut quant_mapping = Vec::new();

    // up to 4 components
    // index with
    // [is_dc][component]
    let mut huffman_table: [[HuffmanTree; 2]; 2] = [
        [HuffmanTree::new(), HuffmanTree::new()],
        [HuffmanTree::new(), HuffmanTree::new()],
    ];

    loop {
        // read >H (python unpack notation), this means
        // we read a big-endian unsigned short.

        if reader.read_exact(&mut buf).is_err() {
            break;
        }

        // Very tiny optimization idea: avoid swapping bytes when
        // reading the marker by just comparing the bytes already
        // swapped (on little endian). On big endian, compare the
        // bytes as normal. No swapping required either way.
        let marker = u16::from_be_bytes(buf);

        println!("{}", get_jpeg_segment_name(marker));

        // TODO: make this a more strongly-typed enum.
        // and make a function like segment_name or something,
        // which returns Option<Marker>
        match marker {
            // start of sequence
            JPEG_START_OF_IMAGE => {}
            JPEG_END_OF_IMAGE => {}
            // Start of scan (actual entropy coded image data)
            JPEG_START_OF_SCAN => {
                // What the hell is this length for?
                let len = read_u16(&mut reader)?;

                reader.seek_relative((len - 2) as i64)?;

                // Any time we encounter 0xFF00, it's just 0xFF.
                // So we basically need to remove any 0 bytes, I guess.

                // Keep reading bytes
                // If you encounter 0xFF, if the next byte is 0x00,
                // just remove the 0x00 part.
                // Otherwise, if there's any other byte afterwards,
                // break out of the loop (0xFFD9).

                // Byte can occur at any position.

                let mut data = vec![];

                let mut prev_byte_was_0xff = false;

                // Is memmap worth looking into?
                // What's the fastest way to do file I/O?
                // Any way to avoid copying from inner buffer
                // of BufReader?
                // Memory is probably a huge bottleneck.

                let mut skipped_bytes = 0;

                // Uhh is there a way to do this that isn't really slow?
                // Might have to use memchr or something.
                // But a continuous memchr that actually marks all fucking
                // bytes instead of inefficiently stopping.
                loop {
                    // May or may not actually use the byte in each iter of the loop.
                    let byte = read_u8(&mut reader)?;
                    if prev_byte_was_0xff {
                        if byte == 0x00 {
                            // push previous byte (since current one is 0x00)
                            data.push(0xFF);
                            skipped_bytes += 1;
                            prev_byte_was_0xff = false;
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        if byte == 0xFF {
                            // will be added in next loop iteration
                            prev_byte_was_0xff = true;
                        } else {
                            data.push(byte);
                        }
                    }
                }

                let mut bitreader = BitReader::new(&data);

                // decode luma DC coefficient
                // Get length of first coefficient

                let dc_bits = huffman_table[1][0].read_code(&mut bitreader);

                // get N bits
                let dc_val = bitreader.get_n_bits(dc_bits as u32).unwrap();

                let mut prev_dc_coeff = 0;

                let dc_coeff = sign_code(dc_bits as u32, dc_val) + prev_dc_coeff;

                println!("DC coeff: {dc_coeff}");

                // first couple of DC coefficients:
                // -87, -41, 12, -51, -54, 15, -64, -51, 14

                // decode AC coefficients

                // first few AC coefficients
                // 25, 7, 1, 3, 1, -1, -2, -5, 1, 1, -1, 1

                // before de-zigzag
                let mut mcu_block = [0; 8 * 8];
                mcu_block[0] = dc_coeff;

                let mut idx = 1;
                loop {
                    let symbol = huffman_table[0][0].read_code(&mut bitreader);

                    // how many bits to read
                    let ac_bits = symbol & 0xf;

                    // how many preceeding zeros there are before this coefficient
                    let run_length = symbol >> 4;

                    let ac_val = bitreader.get_n_bits(ac_bits as u32).unwrap();
                    // is this the final AC coefficient? like, nothing else to do?
                    let ac_coeff = sign_code(ac_bits as u32, ac_val);

                    idx += run_length as usize;
                    mcu_block[idx] = ac_coeff;

                    idx += 1;

                    println!("AC coeff: {ac_coeff}");

                    // if ac_coeff == 0,
                    // that apparently indicates the end of the block.
                    if ac_coeff == 0 {
                        break;
                    }
                }

                println!("coeffs: {:?}", mcu_block);

                // TODO how to know when to stop decoding AC coefficients?
                // also like, how exactly is everything actually laid out?

                // println!("First AC coefficient: {ac_coeff}");

                println!("[BYTE STREAM] data len: {} bytes", data.len());
                println!("[BYTE STREAM]  skipped: {} bytes", skipped_bytes);
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

                // index 0 or 1 only are allowed
                assert!(dst <= 1);

                // if upper 4 bits are 0, 8-bit
                // otherwise 16-bit
                let qt_is_8_bit = (qt_info & 0xf0) == 0;

                // for now we assume 8-bit, since 16-bit requires
                // reading twice as many bytes (roughly).
                assert!(qt_is_8_bit);

                // TODO this isn't correct for 16-bit
                reader.read_exact(&mut quant_matrices[dst as usize])?;

                println!("Quant Matrix: {}-bit", if qt_is_8_bit { "8" } else { "16" });
                print_dst_quant_table(dst);
                print_8x8_quant_table(&quant_matrices[dst as usize]);
                println!();
            }
            JPEG_DEFINE_HUFFMAN_TABLE => {
                // Does jpeg require the huffman tables to be specified
                // in increasing component order?

                // Up to 4 huffman tables are allowed in JPEG

                // Not actually needed, but we do have to advance forward 2 bytes.
                let _len = read_u16(&mut reader)?;

                let ht_info = read_u8(&mut reader)?;

                let ht_num = ht_info & 0xf;
                assert!(ht_num <= 1);

                // bit index 4 (5th bit) specifies whether table is for AC/DC
                // 0 = DC, 1 = AC
                let is_dc = (ht_info >> 4) & 1 == 0;

                // TODO maybe make a build flag for extra checks or something
                // ensure bit index 5-7 is 0
                assert!(ht_info & 0b1110_0000 == 0);

                // I think component 0 is luma
                // and component 1 is chroma

                println!(
                    "Component {ht_num}, {} huffman tree",
                    if is_dc { "DC" } else { "AC" }
                );

                // read 16 bytes for child node counts for 16 levels of huffman tree
                let mut buf = [0; 16];

                reader.read_exact(&mut buf)?;

                let mut code = 0u16;
                let mut bits = 0;

                let mut ht = HuffmanTree {
                    lookup: HashMap::new(),
                };

                for tdepth in buf {
                    code <<= 1;
                    bits += 1;

                    // TODO optimize symbol decoding
                    for _ in 0..tdepth {
                        let symbol = read_u8(&mut reader)?;

                        ht.lookup.insert(HuffmanCode { code, bits }, symbol);

                        code += 1;
                    }
                }

                // so AC is actually stored at index 0,
                // DC tree at index 1
                huffman_table[is_dc as usize][ht_num as usize] = ht;

                // TODO for check_decoder, ensure symbols read equals
                // sum of symbols read, and complies with the length
            }
            // Other currently unsupported marker
            JPEG_START_OF_FRAME => {
                let _len = read_u16(&mut reader)?;

                // bits per sample
                let data_precision = read_u8(&mut reader)?;

                let height = read_u16(&mut reader)?;
                let width = read_u16(&mut reader)?;

                // So number of quant tables is either 1 or 3
                let num_components = read_u8(&mut reader)?;
                assert!([1, 3].contains(&num_components));

                println!(" {}-bit precision", data_precision);
                println!(" Resolution: {width}x{height} px");
                if num_components == 1 {
                    println!(" Monochrome (1 component)");
                } else {
                    println!(" YCbCr or YIQ (3 components)");
                }

                let comp_id = |id: u8| match id {
                    1 => "Y",
                    2 => "Cb",
                    3 => "Cr",
                    4 => "I",
                    5 => "Q",
                    _ => panic!("unknown component id"),
                };

                let qt = |n: u8| match n {
                    0 => "Luminance",
                    1 => "Chrominance",
                    _ => panic!("invalid quant table index"),
                };

                let dashes = || println!(" --------------------");

                let mut buf = [0; 3];
                for _ in 0..num_components {
                    reader.read_exact(&mut buf)?;

                    let vdec = buf[1] & 0xf;
                    let hdec = buf[1] & 0xf0;

                    dashes();
                    println!("     Component ID: {} ({})", buf[0], comp_id(buf[0]));

                    // TODO how exactly are you supposed to actually parse this sample factors stuff?
                    // println!(" Sampling Factors: 4:{}:{}", vdec, hdec);
                    // What ?
                    println!(" Sampling Factors: {}", buf[1]);
                    println!("      Quant Table: {}", qt(buf[2]));
                    // TODO append this to some kind of variable, apparently we need it
                    // something like quant_mapping, an array with [0,1,1]

                    quant_mapping.push(buf[2]);
                }

                dashes();
            }
            _ => {
                // read another BE u16, which indicates the length
                let len = read_u16(&mut reader)?;

                // The readed length includes the size of itself,
                // but since we advanced the reader 2 bytes to actually
                // read the length, we need to subtract by 2 to seek
                // by the correct amount.
                reader.seek_relative((len - 2) as i64)?;
            }
        }
    }

    Ok(())
}
