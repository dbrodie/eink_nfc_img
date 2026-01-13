//! IsoDep BWRY protocol implementation for 4-color e-ink displays
//!
//! Protocol reverse-engineered from the official Android app.
//! See research_docs/PROTOCOL_IsoDep_BWRY.md for detailed documentation.

use core::cell::UnsafeCell;
use core::ptr::null_mut;
use flipperzero_sys as sys;

use crate::protocol_common::{
    self, commands as common_commands, log_error, log_info,
    NfcError, NfcResult, CHUNK_SIZE, IMAGE_DATA_SIZE, NUM_PACKETS,
};

/// BWRY-specific command sequences
pub mod commands {
    /// Display initialization: 74 00 15 00 00
    pub const DISPLAY_INIT: &[u8] = &[0x74, 0x00, 0x15, 0x00, 0x00];

    /// Start data transmission: 74 01 15 01 00
    pub const START_TX: &[u8] = &[0x74, 0x01, 0x15, 0x01, 0x00];

    /// Trigger display refresh: 74 02 15 02 00
    pub const REFRESH: &[u8] = &[0x74, 0x02, 0x15, 0x02, 0x00];

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

/// Protocol handler for BWRY (4-color) NFC e-ink displays
pub struct BwryProtocol {
    nfc: *mut sys::Nfc,
    poller: *mut sys::NfcPoller,
    context: UnsafeCell<PollerContext>,
    result: NfcResult<()>,
}

impl BwryProtocol {
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
    /// This executes the full BWRY protocol sequence:
    /// 1. Initialize communication
    /// 2. Configure display registers (E0, E6, A5)
    /// 3. Transfer image data in 64-byte chunks
    /// 4. Trigger display refresh
    /// 5. Wait for refresh to complete
    /// 6. Cleanup registers
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

    /// NFC poller callback - implements the BWRY protocol state machine
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
                    log_info!("Tag detected! Starting BWRY protocol...");
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
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, common_commands::INIT) {
                        ctx.state = PollerState::Gpio0;
                    } else {
                        log_error!("Init command failed!");
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Gpio0 => {
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, common_commands::GPIO_0) {
                        sys::furi_delay_ms(50);
                        ctx.state = PollerState::Gpio1;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Gpio1 => {
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, common_commands::GPIO_1) {
                        sys::furi_delay_ms(200); // BWRY uses 200ms delay
                        ctx.state = PollerState::DisplayInit;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::DisplayInit => {
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, commands::DISPLAY_INIT) {
                        sys::furi_delay_ms(100);
                        ctx.state = PollerState::RegE0Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegE0Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_E0) {
                        ctx.state = PollerState::RegE0Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegE0Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_E0_VAL) {
                        ctx.state = PollerState::RegE6Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegE6Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_E6) {
                        ctx.state = PollerState::RegE6Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegE6Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_E6_VAL) {
                        ctx.state = PollerState::RegA5Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegA5Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_A5) {
                        ctx.state = PollerState::RegA5Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::RegA5Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_A5_VAL) {
                        sys::furi_delay_ms(100);
                        ctx.state = PollerState::StartTx;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::StartTx => {
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, commands::START_TX) {
                        ctx.state = PollerState::SendData(0);
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::SendData(packet_idx) => {
                    let offset = packet_idx * CHUNK_SIZE;
                    let remaining = IMAGE_DATA_SIZE - offset;
                    let chunk_len = core::cmp::min(CHUNK_SIZE, remaining);

                    if protocol_common::send_image_packet_raw(
                        poller, ctx.tx_buf, ctx.rx_buf,
                        ctx.image_data, offset, chunk_len
                    ) {
                        if packet_idx + 1 >= NUM_PACKETS {
                            // Brief delay after final packet before refresh
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
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, commands::REFRESH) {
                        ctx.state = PollerState::WaitRefresh;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::WaitRefresh => {
                    // Wait for initial refresh (10 seconds for BWRY)
                    sys::furi_delay_ms(10000);
                    ctx.state = PollerState::PollStatus;
                }
                PollerState::PollStatus => {
                    // Poll busy status
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, common_commands::READ_STATUS) {
                        // Response format: [STATUS_BYTE, SW1, SW2]
                        // STATUS_BYTE: 0x00 = busy, non-zero = ready (BWRY)
                        let rx_size = sys::bit_buffer_get_size_bytes(ctx.rx_buf);
                        if rx_size >= 3 {
                            let status_byte = sys::bit_buffer_get_byte(ctx.rx_buf, 0);
                            log_info!("Status poll: byte={:02X}", status_byte);
                            if status_byte != 0x00 {
                                log_info!("Display ready!");
                                ctx.state = PollerState::Cleanup02Select;
                            } else {
                                // Still busy, wait and poll again (400ms for BWRY)
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
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_02) {
                        ctx.state = PollerState::Cleanup02Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Cleanup02Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_02_VAL) {
                        sys::furi_delay_ms(200);
                        ctx.state = PollerState::Cleanup07Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Cleanup07Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_07) {
                        ctx.state = PollerState::Cleanup07Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Cleanup07Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_07_VAL) {
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
}

impl Drop for BwryProtocol {
    fn drop(&mut self) {
        self.cleanup();
    }
}
