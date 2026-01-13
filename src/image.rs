//! Image loading and e-ink format handling
//!
//! Supports loading 8-bit indexed BMP files from SD card and encoding them
//! for different e-ink display formats (BWR 3-color, BWRY 4-color).

use alloc::boxed::Box;
use alloc::vec;
use core::ffi::c_char;
use core::marker::PhantomData;
use flipperzero_sys as sys;

use crate::protocol_common::{DISPLAY_HEIGHT, DISPLAY_WIDTH, IMAGE_DATA_SIZE};
use crate::tag_type::{Bwr, Bwry, ImageFormat};

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

/// Marker trait for image formats
pub trait ImageFormatMarker {
    /// Runtime format identifier
    const FORMAT: ImageFormat;
    /// Total data size in bytes
    const DATA_SIZE: usize;
}

impl ImageFormatMarker for Bwry {
    const FORMAT: ImageFormat = ImageFormat::Bwry;
    const DATA_SIZE: usize = IMAGE_DATA_SIZE; // 200*200*2bits/8 = 10,000
}

impl ImageFormatMarker for Bwr {
    const FORMAT: ImageFormat = ImageFormat::Bwr;
    const DATA_SIZE: usize = IMAGE_DATA_SIZE; // 5,000 B/W + 5,000 Red = 10,000
}

/// Type-safe image container for a specific format
pub struct Image<F: ImageFormatMarker> {
    data: Box<[u8; IMAGE_DATA_SIZE]>,
    _marker: PhantomData<F>,
}

impl<F: ImageFormatMarker> Image<F> {
    /// Get raw pointer to image data
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Get image data as byte slice
    pub fn as_slice(&self) -> &[u8; IMAGE_DATA_SIZE] {
        &self.data
    }
}

/// Runtime-dispatched image container
pub enum AnyImage {
    /// BWRY 4-color image
    Bwry(Image<Bwry>),
    /// BWR 3-color image
    Bwr(Image<Bwr>),
}

/// Calculate squared distance between two RGB colors
fn color_distance_sq(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> u32 {
    let dr = r1 as i32 - r2 as i32;
    let dg = g1 as i32 - g2 as i32;
    let db = b1 as i32 - b2 as i32;
    (dr * dr + dg * dg + db * db) as u32
}

/// Map an RGB color to 2-bit BWRY e-ink color code
/// 0=Black, 1=White, 2=Yellow, 3=Red
fn map_to_bwry_color(r: u8, g: u8, b: u8) -> u8 {
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

/// Map an RGB color to BWR encoding
/// Returns (is_white, is_red) for dual-buffer encoding
fn map_to_bwr_color(r: u8, g: u8, b: u8) -> (bool, bool) {
    let black_dist = color_distance_sq(r, g, b, 0, 0, 0);
    let white_dist = color_distance_sq(r, g, b, 255, 255, 255);
    let red_dist = color_distance_sq(r, g, b, 255, 0, 0);

    let min_dist = black_dist.min(white_dist).min(red_dist);

    if min_dist == white_dist {
        (true, false)  // White: BW=1, Red=0
    } else if min_dist == red_dist {
        (false, true)  // Red: BW=0, Red=1
    } else {
        (false, false) // Black: BW=0, Red=0
    }
}

/// Read and validate BMP headers, returning file handle and metadata
unsafe fn read_bmp_headers(
    path: *const c_char,
) -> ImageResult<(*mut sys::File, *mut sys::Storage, usize, bool)> {
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
    let _pixel_offset = u32::from_le_bytes([file_header[10], file_header[11], file_header[12], file_header[13]]) as usize;

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

    // BMP can be bottom-up (positive height) or top-down (negative height)
    let bottom_up = height > 0;

    // BMP rows are padded to 4-byte boundaries
    let row_size = (DISPLAY_WIDTH + 3) & !3;

    Ok((file, storage, row_size, bottom_up))
}

/// Close BMP file and release resources
unsafe fn close_bmp_file(file: *mut sys::File, storage: *mut sys::Storage) {
    sys::storage_file_close(file);
    sys::storage_file_free(file);
    let _ = storage; // storage is from furi_record_open
    sys::furi_record_close(c_str!("storage"));
}

/// Load an 8-bit indexed BMP file and encode as BWRY 4-color
pub fn load_bmp_bwry(path: *const c_char) -> ImageResult<Image<Bwry>> {
    unsafe {
        let (file, storage, row_size, bottom_up) = read_bmp_headers(path)?;

        // Read color palette (256 entries x 4 bytes each = 1024 bytes)
        let palette_size = 256 * 4;
        let mut palette = vec![0u8; palette_size];
        let read = sys::storage_file_read(file, palette.as_mut_ptr() as *mut _, palette_size);
        if read != palette_size {
            close_bmp_file(file, storage);
            return Err(ImageError::ReadFailed);
        }

        // Build BWRY color code lookup table from palette
        let mut color_map = vec![0u8; 256];
        for i in 0..256 {
            let b = palette[i * 4];
            let g = palette[i * 4 + 1];
            let r = palette[i * 4 + 2];
            color_map[i] = map_to_bwry_color(r, g, b);
        }

        // Allocate output buffer
        let mut data = Box::new([0u8; IMAGE_DATA_SIZE]);

        // Read pixel data row by row
        let mut row_buffer = vec![0u8; row_size];

        for row in 0..DISPLAY_HEIGHT {
            let read = sys::storage_file_read(file, row_buffer.as_mut_ptr() as *mut _, row_size);
            if read != row_size {
                close_bmp_file(file, storage);
                return Err(ImageError::ReadFailed);
            }

            // Determine output row based on orientation
            let out_row = if bottom_up {
                DISPLAY_HEIGHT - 1 - row
            } else {
                row
            };

            // Pack 4 pixels per byte (2 bits each, MSB first)
            let bytes_per_row = DISPLAY_WIDTH / 4; // 50 bytes
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

        close_bmp_file(file, storage);

        Ok(Image {
            data,
            _marker: PhantomData,
        })
    }
}

/// Load an 8-bit indexed BMP file and encode as BWR 3-color (dual buffer)
pub fn load_bmp_bwr(path: *const c_char) -> ImageResult<Image<Bwr>> {
    unsafe {
        let (file, storage, row_size, bottom_up) = read_bmp_headers(path)?;

        // Read color palette (256 entries x 4 bytes each = 1024 bytes)
        let palette_size = 256 * 4;
        let mut palette = vec![0u8; palette_size];
        let read = sys::storage_file_read(file, palette.as_mut_ptr() as *mut _, palette_size);
        if read != palette_size {
            close_bmp_file(file, storage);
            return Err(ImageError::ReadFailed);
        }

        // Build BWR color maps from palette (is_white, is_red)
        let mut bw_map = vec![false; 256];
        let mut red_map = vec![false; 256];
        for i in 0..256 {
            let b = palette[i * 4];
            let g = palette[i * 4 + 1];
            let r = palette[i * 4 + 2];
            let (is_white, is_red) = map_to_bwr_color(r, g, b);
            bw_map[i] = is_white;
            red_map[i] = is_red;
        }

        // Allocate output buffer
        // First 5000 bytes: B/W buffer (white=1, black=0)
        // Second 5000 bytes: Red buffer (red=1, not-red=0)
        let mut data = Box::new([0u8; IMAGE_DATA_SIZE]);

        // Read pixel data row by row
        let mut row_buffer = vec![0u8; row_size];

        for row in 0..DISPLAY_HEIGHT {
            let read = sys::storage_file_read(file, row_buffer.as_mut_ptr() as *mut _, row_size);
            if read != row_size {
                close_bmp_file(file, storage);
                return Err(ImageError::ReadFailed);
            }

            // Determine output row based on orientation
            let out_row = if bottom_up {
                DISPLAY_HEIGHT - 1 - row
            } else {
                row
            };

            // Pack 8 pixels per byte (1 bit each, MSB first)
            let bytes_per_row = DISPLAY_WIDTH / 8; // 25 bytes
            for x_byte in 0..bytes_per_row {
                let mut bw_byte: u8 = 0;
                let mut red_byte: u8 = 0;

                for bit in 0..8 {
                    let x = x_byte * 8 + bit;
                    let palette_idx = row_buffer[x] as usize;

                    bw_byte <<= 1;
                    red_byte <<= 1;

                    if bw_map[palette_idx] {
                        bw_byte |= 1; // White pixel
                    }
                    if red_map[palette_idx] {
                        red_byte |= 1; // Red pixel
                    }
                }

                // B/W buffer: first 5000 bytes
                data[out_row * bytes_per_row + x_byte] = bw_byte;
                // Red buffer: second 5000 bytes
                data[5000 + out_row * bytes_per_row + x_byte] = red_byte;
            }
        }

        close_bmp_file(file, storage);

        Ok(Image {
            data,
            _marker: PhantomData,
        })
    }
}

/// Load a BMP file with runtime format selection
pub fn load_bmp(path: *const c_char, format: ImageFormat) -> ImageResult<AnyImage> {
    match format {
        ImageFormat::Bwry => Ok(AnyImage::Bwry(load_bmp_bwry(path)?)),
        ImageFormat::Bwr => Ok(AnyImage::Bwr(load_bmp_bwr(path)?)),
    }
}
