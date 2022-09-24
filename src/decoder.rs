use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::mem::size_of;

use crate::bitstream::{read_u16, read_u8, BitReader};
use crate::dct::idct;
use crate::ec::{sign_code, to_index, HuffmanCode, HuffmanTree};
use crate::error::DecodeError;

#[derive(Copy, Clone)]
enum JpegMarker {
    StartOfImage,
    ApplicationDefaultHeader,
    DefineQuantizationTable,
    StartOfFrame,
    DefineHuffmanTable,
    StartOfScan,
    EndOfImage,
    PictInfo,
    AdobeApp14,
    Comment,
    AppSeg1,
    AppSeg2,
}

impl JpegMarker {
    fn segment_name(self) -> &'static str {
        match self {
            JpegMarker::StartOfImage => "Start of Image",
            JpegMarker::ApplicationDefaultHeader => "Application Default Header",
            JpegMarker::DefineQuantizationTable => "Define Quantization Table",
            JpegMarker::StartOfFrame => "Start of Frame",
            JpegMarker::DefineHuffmanTable => "Define Huffman Table",
            JpegMarker::StartOfScan => "Start of Scan",
            JpegMarker::EndOfImage => "End of Image",
            JpegMarker::PictInfo => "Picture Info",
            JpegMarker::AdobeApp14 => "Adobe APP14",
            JpegMarker::Comment => "Comment",
            JpegMarker::AppSeg1 => "EXIF Metadata (Application Segment 1)",
            JpegMarker::AppSeg2 => "ICC color profile, FlashPix",
        }
    }
}

struct InvalidJpegMarker {
    marker: u16,
}

impl Debug for InvalidJpegMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid marker, found 0x{:x}", self.marker)
    }
}

impl TryFrom<u16> for JpegMarker {
    type Error = InvalidJpegMarker;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        // TODO: optimization idea, just check if first byte is
        // 0xff and then do lookup table on other byte
        match value {
            0xffd8 => Ok(JpegMarker::StartOfImage),
            0xffe0 => Ok(JpegMarker::ApplicationDefaultHeader),
            0xffdb => Ok(JpegMarker::DefineQuantizationTable),
            0xffc0 => Ok(JpegMarker::StartOfFrame),
            0xffc4 => Ok(JpegMarker::DefineHuffmanTable),
            0xffda => Ok(JpegMarker::StartOfScan),
            0xffd9 => Ok(JpegMarker::EndOfImage),
            0xffec => Ok(JpegMarker::PictInfo),
            0xffee => Ok(JpegMarker::AdobeApp14),
            0xfffe => Ok(JpegMarker::Comment),
            0xffe2 => Ok(JpegMarker::AppSeg2),
            0xffe1 => Ok(JpegMarker::AppSeg1),
            _ => Err(InvalidJpegMarker { marker: value }),
        }
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

#[inline(never)]
pub fn zigzag_descan(coeffs: &[i16; 64]) -> [i16; 64] {
    let mut new = [0; 64];

    for i in 0..64 {
        new[i] = coeffs[ZIGZAG_DECODE_ORDER[i]];
    }

    new
}

pub struct Decoder {
    reader: BufReader<File>,
    d: Dimensions,
}

struct Dimensions {
    w: u16,
    h: u16,
}

fn decode_mcu_block(
    huff_trees: &[[HuffmanTree; 2]; 2],
    quant_matrices: &[[u8; 64]; 2],
    bitreader: &mut BitReader,
    pred: &mut [i16; 3],
) -> [[i16; 64]; 3] {
    // 8x8 blocks stored in this order:
    // Y, Cr, Cb

    // huff tree:
    // [component][is_dc]

    // TODO do not assume 3 components
    let y = decode_matrix(&huff_trees[0], &quant_matrices[0], bitreader, &mut pred[0]);
    let cr = decode_matrix(&huff_trees[1], &quant_matrices[1], bitreader, &mut pred[1]);
    let cb = decode_matrix(&huff_trees[1], &quant_matrices[1], bitreader, &mut pred[2]);

    [y, cr, cb]
}

// Call this function BEFORE doing zigzag descan
#[inline(never)]
fn dequantize(coeffs: &mut [i16; 64], quant_matrix: &[u8; 64]) {
    for i in 0..64 {
        coeffs[i] *= i16::from(quant_matrix[i]);
    }
}

fn decode_matrix(
    huff_trees: &[HuffmanTree; 2],
    quant_matrix: &[u8; 64],
    bitreader: &mut BitReader,
    dc_pred: &mut i16,
) -> [i16; 64] {
    let [ac_huff_tree, dc_huff_tree] = huff_trees;

    // alright so apparently this unwrap is failing because we are just reading an invalid
    // code, not because there are too many bits?
    let dc_bits = dc_huff_tree.read_code(bitreader).unwrap();

    // get N bits
    let dc_val = bitreader.get_n_bits(dc_bits as u32).unwrap();

    let dc_coeff = *dc_pred + sign_code(dc_bits as u32, dc_val);
    *dc_pred = dc_coeff;

    // before de-zigzag
    let mut mcu_block = [0; 8 * 8];
    mcu_block[0] = dc_coeff;

    let mut idx = 1;

    // What besides decoding bits takes up so much time in this function?

    loop {
        let symbol = ac_huff_tree.read_code(bitreader).unwrap();

        // EOB reached
        if symbol == 0 {
            break;
        }

        // how many bits to read
        let ac_bits = symbol & 0xf;

        // how many preceeding zeros there are before this coefficient
        let run_length = symbol >> 4;

        let ac_val = bitreader.get_n_bits(ac_bits as u32).unwrap();
        let ac_coeff = sign_code(ac_bits as u32, ac_val);

        idx += run_length as usize;

        mcu_block[idx] = ac_coeff;

        idx += 1;

        if idx >= 64 {
            break;
        }
    }

    dequantize(&mut mcu_block, quant_matrix);

    // undo zigzag scan order
    let mcu_coeffs = zigzag_descan(&mcu_block);

    mcu_coeffs
}

impl Decoder {
    pub fn new(file: File) -> Self {
        Decoder {
            reader: BufReader::new(file),
            d: Dimensions { w: 0, h: 0 },
        }
    }

    pub fn decode(&mut self) -> Result<(), DecodeError> {
        let mut quant_matrices = [[0u8; 64]; 2];
        let mut quant_mapping = Vec::new();

        let mut out_file = File::create("out.ppm").unwrap();

        let mut blocks = Vec::new();

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

            let marker = JpegMarker::try_from(marker).unwrap();

            println!("{}", marker.segment_name());

            match marker {
                JpegMarker::StartOfImage => {}
                JpegMarker::EndOfImage => {}
                // Start of scan (actual entropy coded image data)
                JpegMarker::StartOfScan => {
                    // What the hell is this length for?
                    let len = read_u16(&mut self.reader)?;

                    self.reader.seek_relative((len - 2) as i64)?;

                    let mut bitreader = BitReader::new(&mut self.reader);

                    let mut dc_pred = [0; 3];

                    for _y in 0..self.d.h / 8 {
                        for _x in 0..self.d.w / 8 {
                            let mcu_block = decode_mcu_block(
                                &huffman_table,
                                &quant_matrices,
                                &mut bitreader,
                                &mut dc_pred,
                            );

                            blocks.push(mcu_block);
                        }
                    }
                }
                JpegMarker::ApplicationDefaultHeader => {
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
                JpegMarker::DefineQuantizationTable => {
                    let mut len = read_u16(&mut self.reader)? as usize - 2;
                    // one DQT can actually define multiple quant tables
                    // so porsche.jpg doesn't decode because it defines 2 quant
                    // tables with one DQT marker

                    'dqt: loop {
                        let qt_info = read_u8(&mut self.reader)?;
                        len -= 1;

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

                        len -= 64;

                        println!("Quant Matrix: {}-bit", if qt_is_8_bit { "8" } else { "16" });
                        print_dst_quant_table(dst);
                        print_8x8_matrix(&quant_matrices[dst as usize]);
                        println!();

                        if len == 0 {
                            break 'dqt;
                        }
                    }
                }
                JpegMarker::DefineHuffmanTable => {
                    // Up to 4 huffman tables are allowed in JPEG

                    // Not actually needed, but we do have to advance forward 2 bytes.
                    let mut len = read_u16(&mut self.reader)? - 2;

                    'dht: loop {
                        let ht_info = read_u8(&mut self.reader)?;
                        len -= 1;

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
                        len -= 16;

                        let mut code = 0u16;
                        let mut bits = 0u32;

                        let mut ht = HuffmanTree::new();

                        for tdepth in buf {
                            code <<= 1;
                            bits += 1;

                            // TODO optimize symbol decoding
                            for _ in 0..tdepth {
                                let symbol = read_u8(&mut self.reader)?;
                                len -= 1;

                                ht.lookup[to_index(code, bits)] = (
                                    HuffmanCode {
                                        bits: bits as u8,
                                        code,
                                    },
                                    symbol,
                                );

                                code += 1;
                            }
                        }

                        // so AC is actually stored at index 0,
                        // DC tree at index 1
                        huffman_table[ht_num as usize][is_dc as usize] = ht;

                        if len == 0 {
                            break 'dht;
                        }
                    }

                    // TODO for check_decoder, ensure symbols read equals
                    // sum of symbols read, and complies with the length
                }
                // Other currently unsupported marker
                JpegMarker::StartOfFrame => {
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

                    out_file.write_all(format!("P6\n{width} {height}\n255\n").as_bytes())?;

                    self.d.w = width;
                    self.d.h = height;
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

        let mut buf = vec![0; 3 * self.d.w as usize * self.d.h as usize];

        let bh = (self.d.h / 8) as usize;
        let bw = (self.d.w / 8) as usize;

        // TODO refactor this into another function
        // so we can actually see wtf is taking up the time according
        // to perf.
        fn ycbcr_to_rgb(y: f64, cb: f64, cr: f64) -> [u8; 3] {
            let r = f64::mul_add(1.402, cr - 128.0, y);
            let g = f64::mul_add(-0.71414, cr - 128.0, f64::mul_add(-0.34414, cb - 128.0, y));
            let b = f64::mul_add(1.772, cb - 128.0, y);

            let r = r as u8;
            let g = g as u8;
            let b = b as u8;

            [r, g, b]
        }

        for y in 0..bh {
            for x in 0..bw {
                let block = blocks[y * bw + x];

                let mut coeffs = [[0.0; 64]; 3];

                let mut out = [[0.0; 64]; 3];

                // cast dct coefficients to f64
                for p in 0..3 {
                    for i in 0..64 {
                        coeffs[p][i] = block[p][i] as f64;
                    }
                }

                for p in 0..3 {
                    idct(&coeffs[p], &mut out[p]);
                }

                for y2 in 0..8 {
                    for x2 in 0..8 {
                        let yp = out[0][y2 * 8 + x2] + 128.0;
                        let cb = out[1][y2 * 8 + x2] + 128.0;
                        let cr = out[2][y2 * 8 + x2] + 128.0;

                        let px = ycbcr_to_rgb(yp, cb, cr);

                        buf[3 * (y * bw * 8 * 8 + 8 * x + y2 * self.d.w as usize + x2)..][..3]
                            .copy_from_slice(&px)
                    }
                }
            }
        }

        out_file.write_all(&buf).unwrap();

        Ok(())
    }
}
