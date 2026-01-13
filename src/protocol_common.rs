//! Shared NFC protocol primitives for e-ink display communication
//!
//! This module contains common types, constants, and helper functions
//! used by all protocol implementations.

extern crate alloc;

use alloc::ffi::CString;
use alloc::format;
use alloc::vec::Vec;
use core::ffi::CStr;
use flipperzero_sys as sys;

/// Log tag for debugging
pub const TAG: &CStr = c"EINK_NFC";

/// Helper macro for debug logging
macro_rules! log_info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        if let Ok(c_msg) = CString::new(msg) {
            unsafe {
                sys::furi_log_print_format(
                    sys::FuriLogLevelInfo,
                    $crate::protocol_common::TAG.as_ptr(),
                    c_msg.as_ptr(),
                );
            }
        }
    }};
}

macro_rules! log_error {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        if let Ok(c_msg) = CString::new(msg) {
            unsafe {
                sys::furi_log_print_format(
                    sys::FuriLogLevelError,
                    $crate::protocol_common::TAG.as_ptr(),
                    c_msg.as_ptr(),
                );
            }
        }
    }};
}

pub(crate) use log_error;
pub(crate) use log_info;

/// Display dimensions (shared by 1.54" displays)
pub const DISPLAY_WIDTH: usize = 200;
pub const DISPLAY_HEIGHT: usize = 200;

/// Total image data size (200 * 200 * 2 bits / 8 = 10,000 bytes for BWRY)
/// BWR uses same size: 5,000 B/W + 5,000 Red = 10,000 bytes
pub const IMAGE_DATA_SIZE: usize = 10_000;

/// Chunk size for data transfer
///
/// Note: The original Android app uses 250-byte chunks.
/// Reduced to 64 bytes here due to Flipper Zero's ISO 14443-4 frame size limits.
pub const CHUNK_SIZE: usize = 64;

/// Number of data packets for single-buffer transfer (BWRY)
pub const NUM_PACKETS: usize = (IMAGE_DATA_SIZE + CHUNK_SIZE - 1) / CHUNK_SIZE;

/// Number of data packets per buffer for dual-buffer transfer (BWR)
pub const NUM_PACKETS_PER_BUFFER: usize = (5_000 + CHUNK_SIZE - 1) / CHUNK_SIZE;

/// NFC operation errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NfcError {
    /// Tag detection failed
    DetectFailed,
    /// Command transmission failed
    TransmitFailed,
    /// Allocation failed
    AllocFailed,
}

pub type NfcResult<T> = Result<T, NfcError>;

/// Shared APDU command sequences
pub mod commands {
    /// Authentication/Init: 74 B1 00 00 08 00 11 22 33 44 55 66 77
    pub const INIT: &[u8] = &[
        0x74, 0xB1, 0x00, 0x00, 0x08,
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
    ];

    /// GPIO/Power control step 0: 74 97 00 08 00
    pub const GPIO_0: &[u8] = &[0x74, 0x97, 0x00, 0x08, 0x00];

    /// GPIO/Power control step 1: 74 97 01 08 00
    pub const GPIO_1: &[u8] = &[0x74, 0x97, 0x01, 0x08, 0x00];

    /// Read busy status: 74 9B 00 0F 01
    pub const READ_STATUS: &[u8] = &[0x74, 0x9B, 0x00, 0x0F, 0x01];
}

/// Helper: Send a raw command and check for success
///
/// Returns true if the command was sent successfully and the response
/// indicates success (SW1=0x90, SW2=0x00).
pub unsafe fn send_command(
    poller: *mut sys::Iso14443_4aPoller,
    tx_buf: *mut sys::BitBuffer,
    rx_buf: *mut sys::BitBuffer,
    cmd: &[u8],
) -> bool {
    // Log command (first 6 bytes max for brevity)
    let cmd_preview: Vec<u8> = cmd.iter().take(6).copied().collect();
    log_info!("TX: {:02X?} (len={})", cmd_preview, cmd.len());

    sys::bit_buffer_reset(tx_buf);
    sys::bit_buffer_reset(rx_buf);
    sys::bit_buffer_copy_bytes(tx_buf, cmd.as_ptr(), cmd.len());

    let error = sys::iso14443_4a_poller_send_block(poller, tx_buf, rx_buf);
    if error != sys::Iso14443_4aErrorNone {
        // Error codes: 0=None, 1=NotPresent, 2=Protocol, 3=Timeout
        log_error!("NFC send error code: {}", error.0);
        return false;
    }

    // Log response
    let rx_size = sys::bit_buffer_get_size_bytes(rx_buf);
    if rx_size > 0 {
        let mut rx_bytes = Vec::new();
        for i in 0..core::cmp::min(rx_size, 8) {
            rx_bytes.push(sys::bit_buffer_get_byte(rx_buf, i));
        }
        log_info!("RX: {:02X?} (len={})", rx_bytes, rx_size);
    } else {
        log_info!("RX: empty");
    }

    // Check for success response (0x90 0x00) at the END of response
    // APDU response format is [DATA...] [SW1] [SW2]
    if rx_size >= 2 {
        let sw1 = sys::bit_buffer_get_byte(rx_buf, rx_size - 2);
        let sw2 = sys::bit_buffer_get_byte(rx_buf, rx_size - 1);
        let success = sw1 == 0x90 && sw2 == 0x00;
        if !success {
            log_error!("Bad response: SW1={:02X} SW2={:02X}", sw1, sw2);
        }
        success
    } else {
        true // Some commands may have minimal response
    }
}

/// Helper: Send a select register command (74 99 00 0D 01 REG)
pub unsafe fn send_select_register(
    poller: *mut sys::Iso14443_4aPoller,
    tx_buf: *mut sys::BitBuffer,
    rx_buf: *mut sys::BitBuffer,
    reg: u8,
) -> bool {
    let cmd = [0x74, 0x99, 0x00, 0x0D, 0x01, reg];
    send_command(poller, tx_buf, rx_buf, &cmd)
}

/// Helper: Send a write data command (74 9A 00 0E LEN DATA...)
pub unsafe fn send_write_data(
    poller: *mut sys::Iso14443_4aPoller,
    tx_buf: *mut sys::BitBuffer,
    rx_buf: *mut sys::BitBuffer,
    data: &[u8],
) -> bool {
    let mut cmd = [0u8; 260];
    cmd[0] = 0x74;
    cmd[1] = 0x9A;
    cmd[2] = 0x00;
    cmd[3] = 0x0E;
    cmd[4] = data.len() as u8;
    cmd[5..5 + data.len()].copy_from_slice(data);
    send_command(poller, tx_buf, rx_buf, &cmd[..5 + data.len()])
}

/// Helper: Send an image data packet from a buffer
///
/// `image_data` - pointer to the full image buffer
/// `offset` - byte offset into the buffer
/// `chunk_len` - number of bytes to send in this packet
pub unsafe fn send_image_packet_raw(
    poller: *mut sys::Iso14443_4aPoller,
    tx_buf: *mut sys::BitBuffer,
    rx_buf: *mut sys::BitBuffer,
    image_data: *const u8,
    offset: usize,
    chunk_len: usize,
) -> bool {
    let mut packet = [0u8; 128]; // 5 header + up to 64 data + margin
    packet[0] = 0x74;
    packet[1] = 0x9A;
    packet[2] = 0x00;
    packet[3] = 0x0E;
    packet[4] = chunk_len as u8;

    let src = image_data.add(offset);
    core::ptr::copy_nonoverlapping(src, packet[5..].as_mut_ptr(), chunk_len);

    send_command(poller, tx_buf, rx_buf, &packet[..5 + chunk_len])
}
