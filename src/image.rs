//! Image loading and 4-color e-ink format handling
//!
//! Supports loading 8-bit indexed BMP files from SD card.
//! BMP format allows viewing images in standard image viewers.

use alloc::boxed::Box;
use alloc::vec;
use core::ffi::c_char;
use flipperzero_sys as sys;

use crate::protocol::{DISPLAY_HEIGHT, DISPLAY_WIDTH, IMAGE_DATA_SIZE};

/// Helper macro for C strings
macro_rules! c_str {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const core::ffi::c_char
    };
}

/// Image loading errors
#[derive(Debug, Clone, Copy)]
pub enum ImageError {
    /// Could not open file
    OpenFailed,
    /// Could not read file
    ReadFailed,
    /// Invalid file format
    InvalidFormat,
    /// Wrong image dimensions
    InvalidSize,
}

pub type ImageResult<T> = Result<T, ImageError>;

/// BMP file header size
const BMP_FILE_HEADER_SIZE: usize = 14;
/// BMP info header size (BITMAPINFOHEADER)
const BMP_INFO_HEADER_SIZE: usize = 40;

/// Calculate squared distance between two RGB colors
fn color_distance_sq(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> u32 {
    let dr = r1 as i32 - r2 as i32;
    let dg = g1 as i32 - g2 as i32;
    let db = b1 as i32 - b2 as i32;
    (dr * dr + dg * dg + db * db) as u32
}

/// Map an RGB color to 2-bit e-ink color code
/// 0=Black, 1=White, 2=Yellow, 3=Red
fn map_to_eink_color(r: u8, g: u8, b: u8) -> u8 {
    // Reference colors
    let black_dist = color_distance_sq(r, g, b, 0, 0, 0);
    let white_dist = color_distance_sq(r, g, b, 255, 255, 255);
    let yellow_dist = color_distance_sq(r, g, b, 255, 255, 0);
    let red_dist = color_distance_sq(r, g, b, 255, 0, 0);

    let min_dist = black_dist.min(white_dist).min(yellow_dist).min(red_dist);

    if min_dist == black_dist {
        0
    } else if min_dist == white_dist {
        1
    } else if min_dist == yellow_dist {
        2
    } else {
        3
    }
}

/// Load an 8-bit indexed BMP file from the SD card
///
/// Expected format:
/// - 200x200 pixels
/// - 8-bit indexed color (256 color palette)
/// - Palette should use: Black (0,0,0), White (255,255,255), Yellow (255,255,0), Red (255,0,0)
pub fn load_bmp_file(path: *const c_char) -> ImageResult<Box<[u8; IMAGE_DATA_SIZE]>> {
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

        // Read BMP file header (14 bytes)
        let mut file_header = [0u8; BMP_FILE_HEADER_SIZE];
        let read = sys::storage_file_read(file, file_header.as_mut_ptr() as *mut _, BMP_FILE_HEADER_SIZE);
        if read != BMP_FILE_HEADER_SIZE {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::ReadFailed);
        }

        // Check BMP magic "BM"
        if file_header[0] != b'B' || file_header[1] != b'M' {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::InvalidFormat);
        }

        // Get pixel data offset from file header
        let pixel_offset = u32::from_le_bytes([file_header[10], file_header[11], file_header[12], file_header[13]]) as usize;

        // Read BMP info header (40 bytes)
        let mut info_header = [0u8; BMP_INFO_HEADER_SIZE];
        let read = sys::storage_file_read(file, info_header.as_mut_ptr() as *mut _, BMP_INFO_HEADER_SIZE);
        if read != BMP_INFO_HEADER_SIZE {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::ReadFailed);
        }

        // Parse dimensions
        let width = i32::from_le_bytes([info_header[4], info_header[5], info_header[6], info_header[7]]);
        let height = i32::from_le_bytes([info_header[8], info_header[9], info_header[10], info_header[11]]);
        let bits_per_pixel = u16::from_le_bytes([info_header[14], info_header[15]]);

        // Validate dimensions (height can be negative for top-down DIB)
        let abs_height = height.abs() as usize;
        if width as usize != DISPLAY_WIDTH || abs_height != DISPLAY_HEIGHT {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::InvalidSize);
        }

        // Must be 8-bit indexed
        if bits_per_pixel != 8 {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::InvalidFormat);
        }

        // Read color palette (256 entries x 4 bytes each = 1024 bytes)
        // HEAP ALLOCATED to avoid stack overflow
        let palette_size = 256 * 4;
        let mut palette = vec![0u8; palette_size];
        let read = sys::storage_file_read(file, palette.as_mut_ptr() as *mut _, palette_size);
        if read != palette_size {
            sys::storage_file_close(file);
            sys::storage_file_free(file);
            sys::furi_record_close(c_str!("storage"));
            return Err(ImageError::ReadFailed);
        }

        // Build color code lookup table from palette
        // HEAP ALLOCATED to avoid stack overflow
        let mut color_map = vec![0u8; 256];
        for i in 0..256 {
            let b = palette[i * 4];
            let g = palette[i * 4 + 1];
            let r = palette[i * 4 + 2];
            // alpha at i * 4 + 3 is ignored
            color_map[i] = map_to_eink_color(r, g, b);
        }

        // Allocate output buffer
        let mut data = Box::new([0u8; IMAGE_DATA_SIZE]);

        // BMP rows are padded to 4-byte boundaries
        let row_size = (DISPLAY_WIDTH + 3) & !3; // Round up to multiple of 4

        // Read pixel data row by row
        // HEAP ALLOCATED buffer to avoid stack overflow
        let mut row_buffer = vec![0u8; row_size];

        // BMP can be bottom-up (positive height) or top-down (negative height)
        let bottom_up = height > 0;

        for row in 0..DISPLAY_HEIGHT {
            let read = sys::storage_file_read(file, row_buffer.as_mut_ptr() as *mut _, row_size);
            if read != row_size {
                sys::storage_file_close(file);
                sys::storage_file_free(file);
                sys::furi_record_close(c_str!("storage"));
                return Err(ImageError::ReadFailed);
            }

            // Determine output row based on orientation
            let out_row = if bottom_up {
                DISPLAY_HEIGHT - 1 - row
            } else {
                row
            };

            // Pack 4 pixels per byte (2 bits each, MSB first)
            let bytes_per_row = DISPLAY_WIDTH / 4;
            for x_byte in 0..bytes_per_row {
                let mut byte_val: u8 = 0;
                for bit in 0..4 {
                    let x = x_byte * 4 + bit;
                    let palette_idx = row_buffer[x] as usize;
                    let color_code = color_map[palette_idx];
                    byte_val = (byte_val << 2) | (color_code & 0x03);
                }
                data[out_row * bytes_per_row + x_byte] = byte_val;
            }
        }

        // Cleanup
        sys::storage_file_close(file);
        sys::storage_file_free(file);
        sys::furi_record_close(c_str!("storage"));

        Ok(data)
    }
}
