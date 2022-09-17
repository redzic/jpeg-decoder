use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::mem::size_of;

use crate::bitstream::{read_u16, read_u8, BitReader};
use crate::ec::{sign_code, HuffmanCode, HuffmanTree};
use crate::error::DecodeError;

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
        _ => panic!("invalid jpeg marker, found 0x{:x}", marker),
    }
}

fn print_8x8_matrix<T: Display + Copy>(x: &[T; 64]) {
    for chunk in x.chunks_exact(8) {
        print!("[");
        for &x in chunk {
            print!("{x: >4} ");
        }
        println!(" ]");
    }
}

fn print_dst_quant_table(dst: u8) {
    match dst {
        0 => println!("Luminance"),
        1 => println!("Chrominance"),
        _ => unreachable!("invalid dst for quant matrix"),
    }
}

#[rustfmt::skip]
const _ZIGZAG_ORDER: [usize; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];

#[rustfmt::skip]
const ZIGZAG_DECODE_ORDER: [usize; 64] = [
    0,  1,  5,  6, 14, 15, 27, 28,
    2,  4,  7, 13, 16, 26, 29, 42,
    3,  8, 12, 17, 25, 30, 41, 43,
    9, 11, 18, 24, 31, 40, 44, 53,
   10, 19, 23, 32, 39, 45, 52, 54,
   20, 22, 33, 38, 46, 51, 55, 60,
   21, 34, 37, 47, 50, 56, 59, 61,
   35, 36, 48, 49, 57, 58, 62, 63,
];

pub fn descan_zigzag(coeffs: &[i16; 64]) -> [i16; 64] {
    let mut new = [0; 64];

    for i in 0..64 {
        new[i] = coeffs[ZIGZAG_DECODE_ORDER[i]];
    }

    new
}

pub struct Decoder {
    reader: BufReader<File>,
}

fn decode_mcu_block(
    huff_trees: &[HuffmanTree; 2],
    quant_matrix: &[u8; 64],
    bitreader: &mut BitReader,
) -> [i16; 64] {
    let [ac_huff_tree, dc_huff_tree] = huff_trees;

    let dc_bits = dc_huff_tree.read_code(bitreader).unwrap();

    // get N bits
    let dc_val = bitreader.get_n_bits(dc_bits as u32).unwrap();

    let prev_dc_coeff = 0;

    let dc_coeff = sign_code(dc_bits as u32, dc_val) + prev_dc_coeff;

    // before de-zigzag
    let mut mcu_block = [0; 8 * 8];
    mcu_block[0] = dc_coeff;

    let mut idx = 1;
    loop {
        let symbol = ac_huff_tree.read_code(bitreader).unwrap();

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

        // this apparently indicates the end of the block.
        if ac_coeff == 0 {
            break;
        }
    }

    // undo zigzag scan order
    let mut mcu_coeffs = descan_zigzag(&mcu_block);

    // dequantize
    // assume luma block for now
    for i in 0..64 {
        mcu_coeffs[i] *= quant_matrix[i] as i16;
    }

    mcu_coeffs
}

impl Decoder {
    pub fn new(file: File) -> Self {
        Decoder {
            reader: BufReader::new(file),
        }
    }

    pub fn decode(&mut self) -> Result<(), DecodeError> {
        let mut quant_matrices = [[0u8; 64]; 2];
        let mut quant_mapping = Vec::new();

        // up to 4 components
        // index with
        // [component][is_dc]
        let mut huffman_table: [[HuffmanTree; 2]; 2] = [
            [HuffmanTree::new(), HuffmanTree::new()],
            [HuffmanTree::new(), HuffmanTree::new()],
        ];

        loop {
            // Very tiny optimization idea: avoid swapping bytes when
            // reading the marker by just comparing the bytes already
            // swapped (on little endian). On big endian, compare the
            // bytes as normal. No swapping required either way.
            let marker = if let Ok(marker) = read_u16(&mut self.reader) {
                marker
            } else {
                break;
            };

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
                    let len = read_u16(&mut self.reader)?;

                    self.reader.seek_relative((len - 2) as i64)?;

                    let mut bitreader = BitReader::new(&mut self.reader);

                    // decode luma DC block

                    let mcu_block =
                        decode_mcu_block(&huffman_table[0], &quant_matrices[0], &mut bitreader);

                    println!("DCT coefficients:");
                    print_8x8_matrix(&mcu_block);

                    // Skip other bytes
                    self.reader.seek(SeekFrom::End(-2))?;
                }
                JPEG_APPLICATION_DEFAULT_HEADER => {
                    let len = read_u16(&mut self.reader)?;

                    let mut null_str = Vec::new();

                    // TODO read len-2 bytes upfront, and search that area instead
                    // of doing it this pretty garbage way

                    // read null-terminated string
                    let n_read = self.reader.read_until(0, &mut null_str)?;
                    assert!(
                        n_read <= len as usize - size_of::<u16>(),
                        "Invalid length after marker in Application Default Header"
                    );

                    let v_maj = read_u8(&mut self.reader)?;
                    let v_min = read_u8(&mut self.reader)?;

                    let units = read_u8(&mut self.reader)?;

                    let dx = read_u16(&mut self.reader)?;
                    let dy = read_u16(&mut self.reader)?;

                    // Thumbnail information
                    let tx = read_u8(&mut self.reader)?;
                    let ty = read_u8(&mut self.reader)?;

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
                    let len = read_u16(&mut self.reader)? as usize - 3;

                    // TODO we handle this incorrectly for 16-bit
                    assert!(len == 64);

                    let qt_info = read_u8(&mut self.reader)?;

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
                    self.reader.read_exact(&mut quant_matrices[dst as usize])?;

                    println!("Quant Matrix: {}-bit", if qt_is_8_bit { "8" } else { "16" });
                    print_dst_quant_table(dst);
                    print_8x8_matrix(&quant_matrices[dst as usize]);
                    println!();
                }
                JPEG_DEFINE_HUFFMAN_TABLE => {
                    // Does jpeg require the huffman tables to be specified
                    // in increasing component order?

                    // Up to 4 huffman tables are allowed in JPEG

                    // Not actually needed, but we do have to advance forward 2 bytes.
                    let _len = read_u16(&mut self.reader)?;

                    let ht_info = read_u8(&mut self.reader)?;

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

                    self.reader.read_exact(&mut buf)?;

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
                            let symbol = read_u8(&mut self.reader)?;

                            ht.lookup.insert(HuffmanCode { code, bits }, symbol);

                            code += 1;
                        }
                    }

                    // so AC is actually stored at index 0,
                    // DC tree at index 1
                    huffman_table[ht_num as usize][is_dc as usize] = ht;

                    // TODO for check_decoder, ensure symbols read equals
                    // sum of symbols read, and complies with the length
                }
                // Other currently unsupported marker
                JPEG_START_OF_FRAME => {
                    let _len = read_u16(&mut self.reader)?;

                    // bits per sample
                    let data_precision = read_u8(&mut self.reader)?;

                    let height = read_u16(&mut self.reader)?;
                    let width = read_u16(&mut self.reader)?;

                    // So number of quant tables is either 1 or 3
                    let num_components = read_u8(&mut self.reader)?;
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
                        self.reader.read_exact(&mut buf)?;

                        // TODO figure out how to use these
                        let _vdec = buf[1] & 0xf;
                        let _hdec = buf[1] & 0xf0;

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
                    let len = read_u16(&mut self.reader)?;

                    // The readed length includes the size of itself,
                    // but since we advanced the reader 2 bytes to actually
                    // read the length, we need to subtract by 2 to seek
                    // by the correct amount.
                    self.reader.seek_relative((len - 2) as i64)?;
                }
            }
        }

        Ok(())
    }
}
