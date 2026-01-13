//! NFC IsoDep protocol implementation for DMPL0154FN1 4-color e-ink display
//!
//! Protocol reverse-engineered from the official Android app.
//! See PROTOCOL.md for detailed command documentation.
//!
//! Updated for flipperzero-rs v0.16.0 NFC API (callback-based poller model).

extern crate alloc;

use alloc::ffi::CString;
use alloc::format;
use core::cell::UnsafeCell;
use core::ffi::CStr;
use core::ptr::null_mut;
use flipperzero_sys as sys;

/// Log tag for debugging
const TAG: &CStr = c"DMPL0154";

/// Helper macro for debug logging
macro_rules! log_info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        if let Ok(c_msg) = CString::new(msg) {
            sys::furi_log_print_format(
                sys::FuriLogLevelInfo,
                TAG.as_ptr(),
                c_msg.as_ptr(),
            );
        }
    }};
}

macro_rules! log_error {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        if let Ok(c_msg) = CString::new(msg) {
            sys::furi_log_print_format(
                sys::FuriLogLevelError,
                TAG.as_ptr(),
                c_msg.as_ptr(),
            );
        }
    }};
}

/// Display dimensions
pub const DISPLAY_WIDTH: usize = 200;
pub const DISPLAY_HEIGHT: usize = 200;

/// Total image data size (200 * 200 * 2 bits / 8 = 10,000 bytes)
pub const IMAGE_DATA_SIZE: usize = 10_000;

/// Chunk size for data transfer
/// Note: Reduced from 250 to 64 bytes due to ISO 14443-4 frame size limits
pub const CHUNK_SIZE: usize = 64;

/// Number of data packets (10000 / 64 = 157, rounded up)
pub const NUM_PACKETS: usize = (IMAGE_DATA_SIZE + CHUNK_SIZE - 1) / CHUNK_SIZE;

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

/// Pre-defined command sequences for DMPL0154FN1
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

    /// Display initialization: 74 00 15 00 00
    pub const DISPLAY_INIT: &[u8] = &[0x74, 0x00, 0x15, 0x00, 0x00];

    /// Start data transmission: 74 01 15 01 00
    pub const START_TX: &[u8] = &[0x74, 0x01, 0x15, 0x01, 0x00];

    /// Trigger display refresh: 74 02 15 02 00
    pub const REFRESH: &[u8] = &[0x74, 0x02, 0x15, 0x02, 0x00];

    /// Read busy status: 74 9B 00 0F 01
    pub const READ_STATUS: &[u8] = &[0x74, 0x9B, 0x00, 0x0F, 0x01];

    /// Register configurations for 4-color mode
    /// Register 0xE0 = 0x02
    pub const REG_E0: u8 = 0xE0;
    pub const REG_E0_VAL: &[u8] = &[0x02];

    /// Register 0xE6 = 0x5D
    pub const REG_E6: u8 = 0xE6;
    pub const REG_E6_VAL: &[u8] = &[0x5D];

    /// Register 0xA5 = 0x00
    pub const REG_A5: u8 = 0xA5;
    pub const REG_A5_VAL: &[u8] = &[0x00];

    /// Cleanup register 0x02 = 0x00
    pub const REG_02: u8 = 0x02;
    pub const REG_02_VAL: &[u8] = &[0x00];

    /// Cleanup register 0x07 = 0xA5
    pub const REG_07: u8 = 0x07;
    pub const REG_07_VAL: &[u8] = &[0xA5];
}

/// State machine states for the poller callback
#[derive(Debug, Clone, Copy, PartialEq)]
enum PollerState {
    WaitingForTag,
    Init,
    Gpio0,
    Gpio1,
    DisplayInit,
    RegE0Select,
    RegE0Write,
    RegE6Select,
    RegE6Write,
    RegA5Select,
    RegA5Write,
    StartTx,
    SendData(usize), // packet index
    Refresh,
    WaitRefresh,
    PollStatus,
    Cleanup02Select,
    Cleanup02Write,
    Cleanup07Select,
    Cleanup07Write,
    Done,
    Error(NfcError),
}

/// Context passed to the NFC poller callback
struct PollerContext {
    state: PollerState,
    image_data: *const u8,
    tx_buf: *mut sys::BitBuffer,
    rx_buf: *mut sys::BitBuffer,
}

/// Protocol handler for DMPL0154FN1 NFC e-ink display
pub struct Dmpl0154Protocol {
    nfc: *mut sys::Nfc,
    poller: *mut sys::NfcPoller,
    context: UnsafeCell<PollerContext>,
    result: NfcResult<()>,
}

impl Dmpl0154Protocol {
    /// Create a new protocol handler
    pub fn new() -> Self {
        Self {
            nfc: null_mut(),
            poller: null_mut(),
            context: UnsafeCell::new(PollerContext {
                state: PollerState::WaitingForTag,
                image_data: null_mut(),
                tx_buf: null_mut(),
                rx_buf: null_mut(),
            }),
            result: Ok(()),
        }
    }

    /// Initialize NFC hardware
    fn init_nfc(&mut self) -> NfcResult<()> {
        unsafe {
            // Allocate NFC instance
            self.nfc = sys::nfc_alloc();
            if self.nfc.is_null() {
                return Err(NfcError::AllocFailed);
            }

            // Allocate poller for ISO14443-4A protocol
            self.poller = sys::nfc_poller_alloc(self.nfc, sys::NfcProtocolIso14443_4a);
            if self.poller.is_null() {
                sys::nfc_free(self.nfc);
                self.nfc = null_mut();
                return Err(NfcError::AllocFailed);
            }

            // Initialize context buffers
            let ctx = &mut *self.context.get();
            ctx.tx_buf = sys::bit_buffer_alloc(512);
            ctx.rx_buf = sys::bit_buffer_alloc(512);

            if ctx.tx_buf.is_null() || ctx.rx_buf.is_null() {
                self.cleanup();
                return Err(NfcError::AllocFailed);
            }

            Ok(())
        }
    }

    /// Clean up NFC resources
    pub fn cleanup(&mut self) {
        unsafe {
            let ctx = &mut *self.context.get();

            if !ctx.tx_buf.is_null() {
                sys::bit_buffer_free(ctx.tx_buf);
                ctx.tx_buf = null_mut();
            }
            if !ctx.rx_buf.is_null() {
                sys::bit_buffer_free(ctx.rx_buf);
                ctx.rx_buf = null_mut();
            }

            if !self.poller.is_null() {
                sys::nfc_poller_free(self.poller);
                self.poller = null_mut();
            }
            if !self.nfc.is_null() {
                sys::nfc_free(self.nfc);
                self.nfc = null_mut();
            }
        }
    }

    /// Write image data to the display
    ///
    /// This executes the full protocol sequence using the NFC poller API:
    /// 1. Initialize communication
    /// 2. Configure display registers
    /// 3. Transfer image data in 250-byte chunks
    /// 4. Trigger display refresh
    /// 5. Wait for refresh to complete
    /// 6. Cleanup
    pub fn write_image(&mut self, image_data: &[u8; IMAGE_DATA_SIZE]) -> NfcResult<()> {
        // Initialize NFC
        self.init_nfc()?;

        unsafe {
            // Set up context
            let ctx = &mut *self.context.get();
            ctx.state = PollerState::WaitingForTag;
            ctx.image_data = image_data.as_ptr();

            // Start poller with callback
            sys::nfc_poller_start(
                self.poller,
                Some(Self::poller_callback),
                self.context.get() as *mut core::ffi::c_void,
            );

            // Wait for completion by polling the state
            // The callback will run on the NFC thread and update the state
            loop {
                sys::furi_delay_ms(100);
                let state = (*self.context.get()).state;
                match state {
                    PollerState::Done => {
                        self.result = Ok(());
                        break;
                    }
                    PollerState::Error(e) => {
                        self.result = Err(e);
                        break;
                    }
                    _ => continue,
                }
            }

            // Stop poller
            sys::nfc_poller_stop(self.poller);
        }

        // Clean up and return result
        self.cleanup();
        self.result
    }

    /// NFC poller callback - implements the protocol state machine
    unsafe extern "C" fn poller_callback(
        event: sys::NfcGenericEvent,
        context: *mut core::ffi::c_void,
    ) -> sys::NfcCommand {
        unsafe {
            let ctx = &mut *(context as *mut PollerContext);

            // Check event data
            let event_data = event.event_data as *const sys::Iso14443_4aPollerEvent;
            if event_data.is_null() {
                return sys::NfcCommandContinue;
            }

            let event_type = (*event_data).type_;

            // Handle WaitingForTag state specially - keep polling on errors
            if ctx.state == PollerState::WaitingForTag {
                if event_type == sys::Iso14443_4aPollerEventTypeReady {
                    // Tag detected! Transition to Init state
                    log_info!("Tag detected! Starting protocol...");
                    ctx.state = PollerState::Init;
                    // Fall through to process Init state
                } else {
                    // Not ready yet (error or other event), keep polling
                    return sys::NfcCommandContinue;
                }
            } else if event_type != sys::Iso14443_4aPollerEventTypeReady {
                // For non-waiting states, errors are fatal
                if event_type == sys::Iso14443_4aPollerEventTypeError {
                    ctx.state = PollerState::Error(NfcError::DetectFailed);
                    return sys::NfcCommandStop;
                }
                return sys::NfcCommandContinue;
            }

            // Get the ISO14443-4A poller instance
            let poller = event.instance as *mut sys::Iso14443_4aPoller;

            // Process state machine
            match ctx.state {
                PollerState::WaitingForTag => {
                    // Should not reach here, but just in case
                    return sys::NfcCommandContinue;
                }
                PollerState::Init => {
                    log_info!("State: Init - sending auth command");
                    if Self::send_command(poller, ctx, commands::INIT) {
                        ctx.state = PollerState::Gpio0;
                    } else {
                        log_error!("Init command failed!");
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Gpio0 => {
                    if Self::send_command(poller, ctx, commands::GPIO_0) {
                        sys::furi_delay_ms(50);
                        ctx.state = PollerState::Gpio1;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Gpio1 => {
                    if Self::send_command(poller, ctx, commands::GPIO_1) {
                        sys::furi_delay_ms(200);
                        ctx.state = PollerState::DisplayInit;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::DisplayInit => {
                    if Self::send_command(poller, ctx, commands::DISPLAY_INIT) {
                        sys::furi_delay_ms(100);
                        ctx.state = PollerState::RegE0Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegE0Select => {
                    if Self::send_select_register(poller, ctx, commands::REG_E0) {
                        ctx.state = PollerState::RegE0Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegE0Write => {
                    if Self::send_write_data(poller, ctx, commands::REG_E0_VAL) {
                        ctx.state = PollerState::RegE6Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegE6Select => {
                    if Self::send_select_register(poller, ctx, commands::REG_E6) {
                        ctx.state = PollerState::RegE6Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegE6Write => {
                    if Self::send_write_data(poller, ctx, commands::REG_E6_VAL) {
                        ctx.state = PollerState::RegA5Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegA5Select => {
                    if Self::send_select_register(poller, ctx, commands::REG_A5) {
                        ctx.state = PollerState::RegA5Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegA5Write => {
                    if Self::send_write_data(poller, ctx, commands::REG_A5_VAL) {
                        sys::furi_delay_ms(100);
                        ctx.state = PollerState::StartTx;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::StartTx => {
                    if Self::send_command(poller, ctx, commands::START_TX) {
                        ctx.state = PollerState::SendData(0);
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::SendData(packet_idx) => {
                    if Self::send_image_packet(poller, ctx, packet_idx) {
                        if packet_idx + 1 >= NUM_PACKETS {
                            sys::furi_delay_ms(50);
                            ctx.state = PollerState::Refresh;
                        } else {
                            ctx.state = PollerState::SendData(packet_idx + 1);
                        }
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Refresh => {
                    if Self::send_command(poller, ctx, commands::REFRESH) {
                        ctx.state = PollerState::WaitRefresh;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::WaitRefresh => {
                    // Wait for initial refresh (10 seconds)
                    sys::furi_delay_ms(10000);
                    ctx.state = PollerState::PollStatus;
                }
                PollerState::PollStatus => {
                    // Poll busy status
                    if Self::send_command(poller, ctx, commands::READ_STATUS) {
                        // Response format: [STATUS_BYTE, SW1, SW2]
                        // STATUS_BYTE: 0x00 = busy, non-zero = ready
                        let rx_size = sys::bit_buffer_get_size_bytes(ctx.rx_buf);
                        if rx_size >= 3 {
                            let status_byte = sys::bit_buffer_get_byte(ctx.rx_buf, 0);
                            log_info!("Status poll: byte={:02X}", status_byte);
                            if status_byte != 0x00 {
                                log_info!("Display ready!");
                                ctx.state = PollerState::Cleanup02Select;
                            } else {
                                // Still busy, wait and poll again
                                sys::furi_delay_ms(400);
                                // Stay in PollStatus state
                            }
                        } else {
                            // Unexpected response length, assume ready
                            log_info!("Unexpected status response len={}, assuming ready", rx_size);
                            ctx.state = PollerState::Cleanup02Select;
                        }
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Cleanup02Select => {
                    if Self::send_select_register(poller, ctx, commands::REG_02) {
                        ctx.state = PollerState::Cleanup02Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Cleanup02Write => {
                    if Self::send_write_data(poller, ctx, commands::REG_02_VAL) {
                        sys::furi_delay_ms(200);
                        ctx.state = PollerState::Cleanup07Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Cleanup07Select => {
                    if Self::send_select_register(poller, ctx, commands::REG_07) {
                        ctx.state = PollerState::Cleanup07Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Cleanup07Write => {
                    if Self::send_write_data(poller, ctx, commands::REG_07_VAL) {
                        ctx.state = PollerState::Done;
                        return sys::NfcCommandStop;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Done | PollerState::Error(_) => {
                    return sys::NfcCommandStop;
                }
            }

            sys::NfcCommandContinue
        }
    }

    /// Helper: Send a raw command and check for success
    unsafe fn send_command(
        poller: *mut sys::Iso14443_4aPoller,
        ctx: &mut PollerContext,
        cmd: &[u8],
    ) -> bool {
        unsafe {
            // Log command (first 6 bytes max for brevity)
            let cmd_preview: alloc::vec::Vec<u8> = cmd.iter().take(6).copied().collect();
            log_info!("TX: {:02X?} (len={})", cmd_preview, cmd.len());

            sys::bit_buffer_reset(ctx.tx_buf);
            sys::bit_buffer_reset(ctx.rx_buf);
            sys::bit_buffer_copy_bytes(ctx.tx_buf, cmd.as_ptr(), cmd.len());

            let error = sys::iso14443_4a_poller_send_block(poller, ctx.tx_buf, ctx.rx_buf);
            if error != sys::Iso14443_4aErrorNone {
                // Error codes: 0=None, 1=NotPresent, 2=Protocol, 3=Timeout
                log_error!("NFC send error code: {}", error.0);
                return false;
            }

            // Log response
            let rx_size = sys::bit_buffer_get_size_bytes(ctx.rx_buf);
            if rx_size > 0 {
                let mut rx_bytes = alloc::vec::Vec::new();
                for i in 0..core::cmp::min(rx_size, 8) {
                    rx_bytes.push(sys::bit_buffer_get_byte(ctx.rx_buf, i));
                }
                log_info!("RX: {:02X?} (len={})", rx_bytes, rx_size);
            } else {
                log_info!("RX: empty");
            }

            // Check for success response (0x90 0x00) at the END of response
            // APDU response format is [DATA...] [SW1] [SW2]
            if rx_size >= 2 {
                let sw1 = sys::bit_buffer_get_byte(ctx.rx_buf, rx_size - 2);
                let sw2 = sys::bit_buffer_get_byte(ctx.rx_buf, rx_size - 1);
                let success = sw1 == 0x90 && sw2 == 0x00;
                if !success {
                    log_error!("Bad response: SW1={:02X} SW2={:02X}", sw1, sw2);
                }
                success
            } else {
                true // Some commands may have minimal response
            }
        }
    }

    /// Helper: Send a select register command
    unsafe fn send_select_register(
        poller: *mut sys::Iso14443_4aPoller,
        ctx: &mut PollerContext,
        reg: u8,
    ) -> bool {
        unsafe {
            let cmd = [0x74, 0x99, 0x00, 0x0D, 0x01, reg];
            Self::send_command(poller, ctx, &cmd)
        }
    }

    /// Helper: Send a write data command
    unsafe fn send_write_data(
        poller: *mut sys::Iso14443_4aPoller,
        ctx: &mut PollerContext,
        data: &[u8],
    ) -> bool {
        unsafe {
            let mut cmd = [0u8; 260];
            cmd[0] = 0x74;
            cmd[1] = 0x9A;
            cmd[2] = 0x00;
            cmd[3] = 0x0E;
            cmd[4] = data.len() as u8;
            cmd[5..5 + data.len()].copy_from_slice(data);
            Self::send_command(poller, ctx, &cmd[..5 + data.len()])
        }
    }

    /// Helper: Send an image data packet
    unsafe fn send_image_packet(
        poller: *mut sys::Iso14443_4aPoller,
        ctx: &mut PollerContext,
        packet_idx: usize,
    ) -> bool {
        unsafe {
            let offset = packet_idx * CHUNK_SIZE;
            // Handle last packet which may be smaller
            let remaining = IMAGE_DATA_SIZE - offset;
            let chunk_len = core::cmp::min(CHUNK_SIZE, remaining);

            let mut packet = [0u8; 128]; // 5 header + up to 64 data + margin
            packet[0] = 0x74;
            packet[1] = 0x9A;
            packet[2] = 0x00;
            packet[3] = 0x0E;
            packet[4] = chunk_len as u8;

            let src = ctx.image_data.add(offset);
            core::ptr::copy_nonoverlapping(src, packet[5..].as_mut_ptr(), chunk_len);

            Self::send_command(poller, ctx, &packet[..5 + chunk_len])
        }
    }
}

impl Drop for Dmpl0154Protocol {
    fn drop(&mut self) {
        self.cleanup();
    }
}
