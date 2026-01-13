//! IsoDep GenB protocol implementation for 3-color (BWR) e-ink displays
//!
//! Protocol reverse-engineered from the official Android app.
//! See research_docs/PROTOCOL_IsoDep_GenB.md for detailed documentation.

use core::cell::UnsafeCell;
use core::ptr::null_mut;
use flipperzero_sys as sys;

use crate::protocol_common::{
    self, commands as common_commands, log_error, log_info,
    NfcError, NfcResult, CHUNK_SIZE, NUM_PACKETS_PER_BUFFER,
};

/// GenB-specific register configurations
pub mod commands {
    /// Register 0x01 = 0xC7 0x00 0x01 (Driver output control)
    pub const REG_01: u8 = 0x01;
    pub const REG_01_VAL: &[u8] = &[0xC7, 0x00, 0x01];

    /// Register 0x11 = 0x01 (Data entry mode)
    pub const REG_11: u8 = 0x11;
    pub const REG_11_VAL: &[u8] = &[0x01];

    /// Register 0x44 = 0x00 0x18 (RAM X address range)
    pub const REG_44: u8 = 0x44;
    pub const REG_44_VAL: &[u8] = &[0x00, 0x18];

    /// Register 0x45 = 0xC7 0x00 0x00 0x00 (RAM Y address range)
    pub const REG_45: u8 = 0x45;
    pub const REG_45_VAL: &[u8] = &[0xC7, 0x00, 0x00, 0x00];

    /// Register 0x3C = 0x05 (Border waveform)
    pub const REG_3C: u8 = 0x3C;
    pub const REG_3C_VAL: &[u8] = &[0x05];

    /// Register 0x18 = 0x80 (Temperature sensor)
    pub const REG_18: u8 = 0x18;
    pub const REG_18_VAL: &[u8] = &[0x80];

    /// Register 0x4E = 0x00 (RAM X address counter)
    pub const REG_4E: u8 = 0x4E;
    pub const REG_4E_VAL: &[u8] = &[0x00];

    /// Register 0x4F = 0xC7 0x00 (RAM Y address counter)
    pub const REG_4F: u8 = 0x4F;
    pub const REG_4F_VAL: &[u8] = &[0xC7, 0x00];

    /// Register 0x24 = B/W data buffer
    pub const REG_BW_DATA: u8 = 0x24;

    /// Register 0x26 = Red data buffer
    pub const REG_RED_DATA: u8 = 0x26;

    /// Register 0x22 = Display update control (write 0xF7 to trigger refresh)
    pub const REG_REFRESH: u8 = 0x22;
    pub const REG_REFRESH_VAL: &[u8] = &[0xF7];

    /// Register 0x20 = Master activation
    pub const REG_ACTIVATE: u8 = 0x20;
}

/// State machine states for the poller callback
#[derive(Debug, Clone, Copy, PartialEq)]
enum PollerState {
    WaitingForTag,
    Init,
    Gpio0,
    Gpio1,
    // Register configuration sequence (8 registers)
    Reg01Select,
    Reg01Write,
    Reg11Select,
    Reg11Write,
    Reg44Select,
    Reg44Write,
    Reg45Select,
    Reg45Write,
    Reg3CSelect,
    Reg3CWrite,
    Reg18Select,
    Reg18Write,
    Reg4ESelect,
    Reg4EWrite,
    Reg4FSelect,
    Reg4FWrite,
    // B/W data transfer
    SelectBwBuffer,
    SendBwData(usize), // packet index
    // Red data transfer
    SelectRedBuffer,
    SendRedData(usize), // packet index
    // Refresh sequence
    Reg22Select,
    Reg22Write,
    Reg20Select,
    WaitRefresh,
    PollStatus,
    Done,
    Error(NfcError),
}

/// Buffer size for B/W or Red data (5000 bytes each)
const BUFFER_SIZE: usize = 5_000;

/// Context passed to the NFC poller callback
struct PollerContext {
    state: PollerState,
    image_data: *const u8,
    tx_buf: *mut sys::BitBuffer,
    rx_buf: *mut sys::BitBuffer,
}

/// Protocol handler for GenB (BWR 3-color) NFC e-ink displays
pub struct GenbProtocol {
    nfc: *mut sys::Nfc,
    poller: *mut sys::NfcPoller,
    context: UnsafeCell<PollerContext>,
    result: NfcResult<()>,
}

impl GenbProtocol {
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
    /// This executes the full GenB protocol sequence:
    /// 1. Initialize communication
    /// 2. Configure display registers (8 register pairs)
    /// 3. Transfer B/W data (5000 bytes) to register 0x24
    /// 4. Transfer Red data (5000 bytes) to register 0x26
    /// 5. Trigger display refresh (write 0xF7 to reg 0x22, select reg 0x20)
    /// 6. Wait for refresh to complete
    ///
    /// Image data layout: First 5000 bytes are B/W buffer, second 5000 bytes are Red buffer
    pub fn write_image(&mut self, image_data: &[u8; 10_000]) -> NfcResult<()> {
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

    /// NFC poller callback - implements the GenB protocol state machine
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
                    log_info!("Tag detected! Starting GenB protocol...");
                    ctx.state = PollerState::Init;
                } else {
                    return sys::NfcCommandContinue;
                }
            } else if event_type != sys::Iso14443_4aPollerEventTypeReady {
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
                        sys::furi_delay_ms(50); // GenB uses 50ms delay
                        ctx.state = PollerState::Gpio1;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Gpio1 => {
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, common_commands::GPIO_1) {
                        sys::furi_delay_ms(50); // GenB uses 50ms delay
                        ctx.state = PollerState::Reg01Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Register 0x01
                PollerState::Reg01Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_01) {
                        ctx.state = PollerState::Reg01Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg01Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_01_VAL) {
                        ctx.state = PollerState::Reg11Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Register 0x11
                PollerState::Reg11Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_11) {
                        ctx.state = PollerState::Reg11Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg11Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_11_VAL) {
                        ctx.state = PollerState::Reg44Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Register 0x44
                PollerState::Reg44Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_44) {
                        ctx.state = PollerState::Reg44Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg44Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_44_VAL) {
                        ctx.state = PollerState::Reg45Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Register 0x45
                PollerState::Reg45Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_45) {
                        ctx.state = PollerState::Reg45Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg45Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_45_VAL) {
                        ctx.state = PollerState::Reg3CSelect;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Register 0x3C
                PollerState::Reg3CSelect => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_3C) {
                        ctx.state = PollerState::Reg3CWrite;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg3CWrite => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_3C_VAL) {
                        ctx.state = PollerState::Reg18Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Register 0x18
                PollerState::Reg18Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_18) {
                        ctx.state = PollerState::Reg18Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg18Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_18_VAL) {
                        ctx.state = PollerState::Reg4ESelect;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Register 0x4E
                PollerState::Reg4ESelect => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_4E) {
                        ctx.state = PollerState::Reg4EWrite;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg4EWrite => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_4E_VAL) {
                        ctx.state = PollerState::Reg4FSelect;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Register 0x4F (last config register, has 100ms delay)
                PollerState::Reg4FSelect => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_4F) {
                        ctx.state = PollerState::Reg4FWrite;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg4FWrite => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_4F_VAL) {
                        sys::furi_delay_ms(100); // Delay after 0x4F write
                        ctx.state = PollerState::SelectBwBuffer;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // B/W data transfer
                PollerState::SelectBwBuffer => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_BW_DATA) {
                        ctx.state = PollerState::SendBwData(0);
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::SendBwData(packet_idx) => {
                    let offset = packet_idx * CHUNK_SIZE;
                    let remaining = BUFFER_SIZE - offset;
                    let chunk_len = core::cmp::min(CHUNK_SIZE, remaining);

                    // B/W data is in the first 5000 bytes
                    if protocol_common::send_image_packet_raw(
                        poller, ctx.tx_buf, ctx.rx_buf,
                        ctx.image_data, offset, chunk_len
                    ) {
                        if packet_idx + 1 >= NUM_PACKETS_PER_BUFFER {
                            ctx.state = PollerState::SelectRedBuffer;
                        } else {
                            ctx.state = PollerState::SendBwData(packet_idx + 1);
                        }
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Red data transfer
                PollerState::SelectRedBuffer => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_RED_DATA) {
                        ctx.state = PollerState::SendRedData(0);
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::SendRedData(packet_idx) => {
                    let offset = packet_idx * CHUNK_SIZE;
                    let remaining = BUFFER_SIZE - offset;
                    let chunk_len = core::cmp::min(CHUNK_SIZE, remaining);

                    // Red data is in the second 5000 bytes (offset by BUFFER_SIZE)
                    if protocol_common::send_image_packet_raw(
                        poller, ctx.tx_buf, ctx.rx_buf,
                        ctx.image_data, BUFFER_SIZE + offset, chunk_len
                    ) {
                        if packet_idx + 1 >= NUM_PACKETS_PER_BUFFER {
                            ctx.state = PollerState::Reg22Select;
                        } else {
                            ctx.state = PollerState::SendRedData(packet_idx + 1);
                        }
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                // Refresh sequence
                PollerState::Reg22Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_REFRESH) {
                        ctx.state = PollerState::Reg22Write;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg22Write => {
                    if protocol_common::send_write_data(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_REFRESH_VAL) {
                        ctx.state = PollerState::Reg20Select;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::Reg20Select => {
                    if protocol_common::send_select_register(poller, ctx.tx_buf, ctx.rx_buf, commands::REG_ACTIVATE) {
                        ctx.state = PollerState::WaitRefresh;
                    } else {
                        ctx.state = PollerState::Error(NfcError::TransmitFailed);
                        return sys::NfcCommandStop;
                    }
                }
                PollerState::WaitRefresh => {
                    // Wait for initial refresh (4 seconds for GenB)
                    sys::furi_delay_ms(4000);
                    ctx.state = PollerState::PollStatus;
                }
                PollerState::PollStatus => {
                    // Poll busy status
                    if protocol_common::send_command(poller, ctx.tx_buf, ctx.rx_buf, common_commands::READ_STATUS) {
                        // Response format: [STATUS_BYTE, SW1, SW2]
                        // STATUS_BYTE: 0x01 = ready (GenB), other values = busy
                        let rx_size = sys::bit_buffer_get_size_bytes(ctx.rx_buf);
                        if rx_size >= 3 {
                            let status_byte = sys::bit_buffer_get_byte(ctx.rx_buf, 0);
                            log_info!("Status poll: byte={:02X}", status_byte);
                            if status_byte == 0x01 {
                                log_info!("Display ready!");
                                ctx.state = PollerState::Done;
                                return sys::NfcCommandStop;
                            } else {
                                // Still busy, wait and poll again (200ms for GenB)
                                sys::furi_delay_ms(200);
                                // Stay in PollStatus state
                            }
                        } else {
                            // Unexpected response length, assume ready
                            log_info!("Unexpected status response len={}, assuming ready", rx_size);
                            ctx.state = PollerState::Done;
                            return sys::NfcCommandStop;
                        }
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

impl Drop for GenbProtocol {
    fn drop(&mut self) {
        self.cleanup();
    }
}
