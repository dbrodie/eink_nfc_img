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
