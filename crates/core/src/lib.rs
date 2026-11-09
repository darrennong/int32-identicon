//! Int32 Identicon protocol V1. See docs/PROTOCOL-V1.md for the wire contract.
mod ecc;
mod locator;
pub mod scan;
#[cfg(target_arch = "wasm32")]
mod wasm;

use std::fmt;

pub const VERSION: u8 = 1;
pub const DEFAULT_IMAGE_SIZE: usize = 640;
pub const MIN_IMAGE_SIZE: usize = 20;
pub const MAX_IMAGE_SIZE: usize = 2560;
const IMAGE_CELLS: usize = 20;
const QUIET_ZONE_CELLS: usize = 2;
pub const PALETTE: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x4b, 0x55, 0x63],
    [0xcb, 0xd5, 0xe1],
    [0xff, 0xff, 0xff],
    [0xb9, 0x1c, 0x1c],
    [0xf9, 0x73, 0x16],
    [0xfa, 0xcc, 0x15],
    [0x84, 0xcc, 0x16],
    [0x15, 0x80, 0x3d],
    [0x0f, 0x76, 0x6e],
    [0x06, 0xb6, 0xd4],
    [0x38, 0xbd, 0xf8],
    [0x1d, 0x4e, 0xd8],
    [0x6d, 0x28, 0xd9],
    [0xa2, 0x1c, 0xaf],
    [0x78, 0x35, 0x0f],
];
pub const FOREGROUNDS: [u8; 9] = [0, 1, 4, 8, 9, 12, 13, 14, 15];
pub const WARM_COOL: [u8; 16] = [0, 1, 2, 3, 15, 4, 5, 6, 14, 7, 8, 9, 10, 11, 13, 12];
/// Mandatory reversible visual XOR mask, applied after ECC.
pub const MASK: [u8; 9] = [0xa5, 0x3c, 0x96, 0xf0, 0x69, 0x5a, 0xc3, 0x87, 0x1e];
pub type Grid = [[u8; 16]; 16];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Foreground,
    Ecc,
    Crc,
    Version,
    Image,
    Quality,
    NotFound,
    Ambiguous,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Foreground => "unsupported foreground",
            Self::Ecc => "uncorrectable codeword",
            Self::Crc => "CRC mismatch",
            Self::Version => "unsupported protocol version",
            Self::Image => "invalid image or quadrilateral",
            Self::Quality => "insufficient image quality",
            Self::NotFound => "no valid identicon found",
            Self::Ambiguous => "multiple distinct valid results",
        })
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoded {
    pub value: i32,
    pub foreground: u8,
    pub repaired_bytes: u8,
}

/// CRC-16/CCITT-FALSE: poly=0x1021, init=0xffff, refin/refout=false, xorout=0.
pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0xffff_u16;
    for &byte in bytes {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub fn default_foreground(value: i32) -> u8 {
    FOREGROUNDS[(value as u32 % 9) as usize]
}

/// Returns the unmasked, systematic nine-byte codeword (MSB first on the grid).
pub fn encode(value: i32, foreground: Option<u8>) -> Result<[u8; 9], Error> {
    let foreground = foreground.unwrap_or_else(|| default_foreground(value));
    if !FOREGROUNDS.contains(&foreground) {
        return Err(Error::Foreground);
    }
    let mut message = [0; 7];
    message[0] = VERSION << 4 | foreground;
    message[1..5].copy_from_slice(&value.to_be_bytes());
    let crc = crc16(&message[..5]);
    message[5..7].copy_from_slice(&crc.to_be_bytes());
    Ok(ecc::encode(message))
}

pub fn decode(word: [u8; 9], erasures: &[usize]) -> Result<Decoded, Error> {
    let (word, repaired_bytes) = ecc::correct(word, erasures)?;
    if word[0] >> 4 != VERSION {
        return Err(Error::Version);
    }
    let foreground = word[0] & 15;
    if !FOREGROUNDS.contains(&foreground) {
        return Err(Error::Foreground);
    }
    if crc16(&word[..5]) != u16::from_be_bytes([word[5], word[6]]) {
        return Err(Error::Crc);
    }
    Ok(Decoded {
        value: i32::from_be_bytes(word[1..5].try_into().unwrap()),
        foreground,
        repaired_bytes,
    })
}

pub fn grid(value: i32, foreground: Option<u8>) -> Result<Grid, Error> {
    let word = encode(value, foreground)?;
    let fg = word[0] & 15;
    let mut result = [[3; 16]; 16];
    for i in 0..16 {
        result[0][i] = i as u8;
        result[i][15] = (15 - i) as u8;
        result[i][0] = WARM_COOL[i];
        result[15][i] = WARM_COOL[15 - i];
    }
    for bit in 0..72 {
        let row = bit / 6 + 2;
        let col = bit % 6 + 2;
        let value = (word[bit / 8] ^ MASK[bit / 8]) & (0x80 >> (bit % 8)) != 0;
        result[row][col] = if value { fg } else { 3 };
        result[row][15 - col] = result[row][col];
    }
    Ok(result)
}

/// Render SVG with the default pixel dimensions.
pub fn svg(value: i32, foreground: Option<u8>) -> Result<String, Error> {
    svg_with_size(value, foreground, DEFAULT_IMAGE_SIZE)
}

/// Render a square SVG with a pixel side length including two white quiet-zone cells.
pub fn svg_with_size(value: i32, foreground: Option<u8>, size: usize) -> Result<String, Error> {
    if !(MIN_IMAGE_SIZE..=MAX_IMAGE_SIZE).contains(&size) {
        return Err(Error::Image);
    }
    let cells = grid(value, foreground)?;
    let mut result = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {IMAGE_CELLS} {IMAGE_CELLS}\" width=\"{size}\" height=\"{size}\" shape-rendering=\"crispEdges\" role=\"img\" aria-label=\"Int32 identicon\"><rect width=\"{IMAGE_CELLS}\" height=\"{IMAGE_CELLS}\" fill=\"white\"/>");
    for (y, row) in cells.iter().enumerate() {
        for (x, &index) in row.iter().enumerate() {
            if index == 3 {
                continue;
            }
            let c = PALETTE[index as usize];
            result.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\" fill=\"#{:02X}{:02X}{:02X}\"/>",
                x + QUIET_ZONE_CELLS,
                y + QUIET_ZONE_CELLS,
                c[0],
                c[1],
                c[2]
            ));
        }
    }
    result.push_str("</svg>\n");
    Ok(result)
}

/// Render lossless RGBA at an integer cell size, with the same quiet zone as SVG.
pub fn rgba(
    value: i32,
    foreground: Option<u8>,
    cell_size: usize,
) -> Result<(usize, Vec<u8>), Error> {
    if !(1..=MAX_IMAGE_SIZE / IMAGE_CELLS).contains(&cell_size) {
        return Err(Error::Image);
    }
    rgba_with_size(value, foreground, IMAGE_CELLS * cell_size)
}

/// Render a square RGBA image with the requested pixel side length, including the quiet zone.
pub fn rgba_with_size(
    value: i32,
    foreground: Option<u8>,
    size: usize,
) -> Result<(usize, Vec<u8>), Error> {
    if !(MIN_IMAGE_SIZE..=MAX_IMAGE_SIZE).contains(&size) {
        return Err(Error::Image);
    }
    let cells = grid(value, foreground)?;
    let mut pixels = vec![255; size * size * 4];
    // Round each cell boundary to the nearest pixel to keep the palette solid at any size.
    let boundary = |cell| (cell * size + IMAGE_CELLS / 2) / IMAGE_CELLS;
    for (y, row) in cells.iter().enumerate() {
        let top = boundary(y + QUIET_ZONE_CELLS);
        let bottom = boundary(y + QUIET_ZONE_CELLS + 1);
        for (x, &index) in row.iter().enumerate() {
            let [r, g, b] = PALETTE[index as usize];
            let color = [r, g, b, 255];
            let left = boundary(x + QUIET_ZONE_CELLS);
            let right = boundary(x + QUIET_ZONE_CELLS + 1);
            for y in top..bottom {
                let row_offset = y * size;
                let (row_pixels, _) =
                    pixels[(row_offset + left) * 4..(row_offset + right) * 4].as_chunks_mut::<4>();
                for pixel in row_pixels {
                    *pixel = color;
                }
            }
        }
    }
    Ok((size, pixels))
}
