//! Image loading and 4-color e-ink format handling
//!
//! Supports loading pre-converted .4ei files from SD card.
//! The .4ei format is a simple binary format optimized for this display.

use alloc::boxed::Box;
use core::ffi::c_char;
use flipperzero_sys as sys;

use crate::protocol::{DISPLAY_HEIGHT, DISPLAY_WIDTH, IMAGE_DATA_SIZE};

/// Helper macro for C strings
macro_rules! c_str {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const core::ffi::c_char
    };
}

/// Magic bytes for .4ei file format
pub const MAGIC: &[u8; 4] = b"4EI1";

/// Header size (magic + width + height)
pub const HEADER_SIZE: usize = 8;

/// 4-color pixel values (2 bits each)
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Color {
    Black = 0b00,
    White = 0b01,
    Yellow = 0b10,
    Red = 0b11,
}

impl Color {
    /// RGB values for color matching during conversion
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            Color::Black => (0, 0, 0),
            Color::White => (255, 255, 255),
            Color::Yellow => (255, 255, 0),
            Color::Red => (255, 0, 0),
        }
    }

    /// Find nearest color using Euclidean RGB distance
    pub fn nearest(r: u8, g: u8, b: u8) -> Self {
        let colors = [Color::Black, Color::White, Color::Yellow, Color::Red];
        let mut best = Color::Black;
        let mut best_dist = u32::MAX;

        for &color in &colors {
            let (cr, cg, cb) = color.rgb();
            let dr = (r as i32 - cr as i32).pow(2) as u32;
            let dg = (g as i32 - cg as i32).pow(2) as u32;
            let db = (b as i32 - cb as i32).pow(2) as u32;
            let dist = dr + dg + db;

            if dist < best_dist {
                best_dist = dist;
                best = color;
            }
        }
        best
    }

    /// Convert from 2-bit value
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0x03 {
            0 => Color::Black,
            1 => Color::White,
            2 => Color::Yellow,
            3 => Color::Red,
            _ => unreachable!(),
        }
    }
}

/// Image loading errors
#[derive(Debug, Clone, Copy)]
pub enum ImageError {
    /// File not found
    FileNotFound,
    /// Could not open file
    OpenFailed,
    /// Could not read file
    ReadFailed,
    /// Invalid file format
    InvalidFormat,
    /// Wrong image dimensions
    InvalidSize,
    /// Memory allocation failed
    AllocFailed,
}

pub type ImageResult<T> = Result<T, ImageError>;

/// Load a .4ei file from the SD card
///
/// File format:
/// - Bytes 0-3: Magic "4EI1"
/// - Bytes 4-5: Width (little-endian u16)
/// - Bytes 6-7: Height (little-endian u16)
/// - Bytes 8+: Image data (10,000 bytes for 200x200)
pub fn load_4ei_file(path: *const c_char) -> ImageResult<Box<[u8; IMAGE_DATA_SIZE]>> {
    unsafe {
        // Open file
        let storage = sys::furi_record_open(c_str!("storage")) as *mut sys::Storage;
        let file = sys::storage_file_alloc(storage);

        if !sys::storage_file_open(
            file,
            path,
            sys::FSAM_READ,
            sys::FSOM_OPEN_EXISTING,
        ) {
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::OpenFailed);
        }

        // Read header
        let mut header = [0u8; HEADER_SIZE];
        let read = sys::storage_file_read(file, header.as_mut_ptr() as *mut _, HEADER_SIZE);
        if read != HEADER_SIZE {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::ReadFailed);
        }

        // Validate magic
        if &header[0..4] != MAGIC {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::InvalidFormat);
        }

        // Validate dimensions
        let width = u16::from_le_bytes([header[4], header[5]]) as usize;
        let height = u16::from_le_bytes([header[6], header[7]]) as usize;

        if width != DISPLAY_WIDTH || height != DISPLAY_HEIGHT {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::InvalidSize);
        }

        // Allocate image buffer
        let mut data = Box::new([0u8; IMAGE_DATA_SIZE]);

        // Read image data
        let read = sys::storage_file_read(
            file,
            data.as_mut_ptr() as *mut _,
            IMAGE_DATA_SIZE,
        );
        if read != IMAGE_DATA_SIZE {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::ReadFailed);
        }

        // Cleanup
        sys::storage_file_close(file);
        sys::storage_file_free(file);
        sys::furi_record_close(c_str!("storage"));

        Ok(data)
    }
}

/// Load a raw binary file (no header, just 10,000 bytes of image data)
pub fn load_raw_file(path: *const c_char) -> ImageResult<Box<[u8; IMAGE_DATA_SIZE]>> {
    unsafe {
        // Open file
        let storage = sys::furi_record_open(c_str!("storage")) as *mut sys::Storage;
        let file = sys::storage_file_alloc(storage);

        if !sys::storage_file_open(
            file,
            path,
            sys::FSAM_READ,
            sys::FSOM_OPEN_EXISTING,
        ) {
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::OpenFailed);
        }

        // Check file size
        let size = sys::storage_file_size(file);
        if size != IMAGE_DATA_SIZE as u64 {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::InvalidSize);
        }

        // Allocate and read
        let mut data = Box::new([0u8; IMAGE_DATA_SIZE]);
        let read = sys::storage_file_read(
            file,
            data.as_mut_ptr() as *mut _,
            IMAGE_DATA_SIZE,
        );
        if read != IMAGE_DATA_SIZE {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::ReadFailed);
        }

        // Cleanup
        sys::storage_file_close(file);
        sys::storage_file_free(file);
        sys::furi_record_close(c_str!("storage"));

        Ok(data)
    }
}

/// Get pixel color at (x, y) from packed image data
pub fn get_pixel(data: &[u8; IMAGE_DATA_SIZE], x: usize, y: usize) -> Color {
    if x >= DISPLAY_WIDTH || y >= DISPLAY_HEIGHT {
        return Color::White;
    }

    let byte_idx = y * (DISPLAY_WIDTH / 4) + (x / 4);
    let bit_offset = 6 - ((x % 4) * 2);
    let value = (data[byte_idx] >> bit_offset) & 0x03;

    Color::from_bits(value)
}

/// Set pixel color at (x, y) in packed image data
pub fn set_pixel(data: &mut [u8; IMAGE_DATA_SIZE], x: usize, y: usize, color: Color) {
    if x >= DISPLAY_WIDTH || y >= DISPLAY_HEIGHT {
        return;
    }

    let byte_idx = y * (DISPLAY_WIDTH / 4) + (x / 4);
    let bit_offset = 6 - ((x % 4) * 2);
    let mask = !(0x03 << bit_offset);

    data[byte_idx] = (data[byte_idx] & mask) | ((color as u8) << bit_offset);
}

/// Create a test pattern image (colored stripes)
pub fn create_test_pattern() -> Box<[u8; IMAGE_DATA_SIZE]> {
    let mut data = Box::new([0u8; IMAGE_DATA_SIZE]);

    for y in 0..DISPLAY_HEIGHT {
        let color = match y / 50 {
            0 => Color::Black,
            1 => Color::White,
            2 => Color::Yellow,
            3 => Color::Red,
            _ => Color::White,
        };

        for x in 0..DISPLAY_WIDTH {
            set_pixel(&mut data, x, y, color);
        }
    }

    data
}
