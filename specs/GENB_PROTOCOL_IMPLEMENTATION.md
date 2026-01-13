# Implementation Plan: Add IsoDep_GenB (BWR 3-color) Support

## Summary

Add support for the **1.54inch e-Paper B** (BWR 3-color) display alongside the existing **1.54inch e-Paper Y** (BWRY 4-color). Uses separate protocol modules and generic image types for type safety.

## Architecture Overview

```
src/
├── main.rs              # UI, uses TagType to select protocol/image format
├── tag_type.rs          # TagType enum with ImageFormat and Protocol enums
├── image.rs             # Generic Image<F> where F: ImageFormat trait
├── protocol_bwry.rs     # BWRY protocol, accepts Image<Bwry>
├── protocol_genb.rs     # GenB protocol, accepts Image<Bwr>
└── protocol_common.rs   # Shared NFC helpers (send_command, etc.)
```

## Type System Design

### `src/tag_type.rs`

```rust
/// Image format marker types
pub struct Bwr;   // 3-color: Black, White, Red
pub struct Bwry;  // 4-color: Black, White, Red, Yellow

/// Image format enum for runtime selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    Bwr,
    Bwry,
}

/// Protocol enum for runtime selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Protocol {
    IsodepBwry,
    IsodepGenb,
}

/// Tag type combining display info, image format, and protocol
#[derive(Debug, Clone, Copy)]
pub struct TagType {
    pub name: &'static str,
    pub width: usize,
    pub height: usize,
    pub image_format: ImageFormat,
    pub protocol: Protocol,
}

impl TagType {
    pub const EPAPER_Y_154: TagType = TagType {
        name: "1.54inch e-Paper Y",
        width: 200,
        height: 200,
        image_format: ImageFormat::Bwry,
        protocol: Protocol::IsodepBwry,
    };

    pub const EPAPER_B_154: TagType = TagType {
        name: "1.54inch e-Paper B",
        width: 200,
        height: 200,
        image_format: ImageFormat::Bwr,
        protocol: Protocol::IsodepGenb,
    };

    pub const ALL: &'static [TagType] = &[
        Self::EPAPER_Y_154,
        Self::EPAPER_B_154,
    ];
}
```

### `src/image.rs`

```rust
use core::marker::PhantomData;
use crate::tag_type::{Bwr, Bwry, ImageFormat};

/// Marker trait for image formats
pub trait ImageFormatMarker {
    const FORMAT: ImageFormat;
    const DATA_SIZE: usize;
}

impl ImageFormatMarker for Bwry {
    const FORMAT: ImageFormat = ImageFormat::Bwry;
    const DATA_SIZE: usize = 10_000;  // 200*200*2bits/8
}

impl ImageFormatMarker for Bwr {
    const FORMAT: ImageFormat = ImageFormat::Bwr;
    const DATA_SIZE: usize = 10_000;  // 5000 B/W + 5000 Red
}

/// Type-safe image container
pub struct Image<F: ImageFormatMarker> {
    data: Box<[u8; 10_000]>,
    _marker: PhantomData<F>,
}

impl<F: ImageFormatMarker> Image<F> {
    pub fn as_ptr(&self) -> *const u8 { self.data.as_ptr() }
    pub fn as_slice(&self) -> &[u8] { &*self.data }
}

/// Load BMP for BWRY format
pub fn load_bmp_bwry(path: *const c_char) -> ImageResult<Image<Bwry>> { ... }

/// Load BMP for BWR format
pub fn load_bmp_bwr(path: *const c_char) -> ImageResult<Image<Bwr>> { ... }

/// Runtime dispatch loader
pub enum AnyImage {
    Bwry(Image<Bwry>),
    Bwr(Image<Bwr>),
}

pub fn load_bmp(path: *const c_char, format: ImageFormat) -> ImageResult<AnyImage> {
    match format {
        ImageFormat::Bwry => Ok(AnyImage::Bwry(load_bmp_bwry(path)?)),
        ImageFormat::Bwr => Ok(AnyImage::Bwr(load_bmp_bwr(path)?)),
    }
}
```

### `src/protocol_common.rs`

Shared NFC primitives extracted from current `protocol.rs`:

```rust
pub const CHUNK_SIZE: usize = 64;

pub enum NfcError { DetectFailed, TransmitFailed, AllocFailed }
pub type NfcResult<T> = Result<T, NfcError>;

/// Shared APDU commands
pub mod commands {
    pub const INIT: &[u8] = &[0x74, 0xB1, 0x00, 0x00, 0x08, ...];
    pub const GPIO_0: &[u8] = &[0x74, 0x97, 0x00, 0x08, 0x00];
    pub const GPIO_1: &[u8] = &[0x74, 0x97, 0x01, 0x08, 0x00];
    pub const READ_STATUS: &[u8] = &[0x74, 0x9B, 0x00, 0x0F, 0x01];
}

/// NFC helper functions
pub unsafe fn send_command(...) -> bool { ... }
pub unsafe fn send_select_register(...) -> bool { ... }
pub unsafe fn send_write_data(...) -> bool { ... }
pub unsafe fn send_image_packet(...) -> bool { ... }
```

### `src/protocol_bwry.rs`

BWRY protocol accepting only `Image<Bwry>`:

```rust
use crate::image::{Image, ImageFormatMarker};
use crate::tag_type::Bwry;
use crate::protocol_common::*;

pub mod commands {
    pub const DISPLAY_INIT: &[u8] = &[0x74, 0x00, 0x15, 0x00, 0x00];
    pub const START_TX: &[u8] = &[0x74, 0x01, 0x15, 0x01, 0x00];
    pub const REFRESH: &[u8] = &[0x74, 0x02, 0x15, 0x02, 0x00];
    // Register values E0, E6, A5, cleanup 02, 07
}

enum PollerState { WaitingForTag, Init, Gpio0, Gpio1, DisplayInit, ... Done, Error(NfcError) }

pub struct BwryProtocol { ... }

impl BwryProtocol {
    pub fn new() -> Self { ... }

    /// Write BWRY image - type system ensures correct format
    pub fn write_image(&mut self, image: &Image<Bwry>) -> NfcResult<()> { ... }
}
```

### `src/protocol_genb.rs`

GenB protocol accepting only `Image<Bwr>`:

```rust
use crate::image::{Image, ImageFormatMarker};
use crate::tag_type::Bwr;
use crate::protocol_common::*;

pub mod commands {
    pub const REG_01: u8 = 0x01;
    pub const REG_01_VAL: &[u8] = &[0xC7, 0x00, 0x01];
    // ... all GenB registers
    pub const REG_BW_DATA: u8 = 0x24;
    pub const REG_RED_DATA: u8 = 0x26;
}

enum PollerState {
    WaitingForTag, Init, Gpio0, Gpio1,
    Reg01Select, Reg01Write, ...,
    SelectBwBuffer, SendBwData(usize),
    SelectRedBuffer, SendRedData(usize),
    Reg22Select, Reg22Write, Reg20Select,
    WaitRefresh, PollStatus,
    Done, Error(NfcError)
}

pub struct GenbProtocol { ... }

impl GenbProtocol {
    pub fn new() -> Self { ... }

    /// Write BWR image - type system ensures correct format
    pub fn write_image(&mut self, image: &Image<Bwr>) -> NfcResult<()> { ... }
}
```

### `src/main.rs`

UI uses `TagType` to dispatch:

```rust
mod tag_type;
mod image;
mod protocol_common;
mod protocol_bwry;
mod protocol_genb;

use tag_type::{TagType, Protocol, ImageFormat};
use image::AnyImage;

struct App {
    // ... existing fields ...
    selected_tag: Option<TagType>,
    image_data: Option<AnyImage>,
}

// Display menu shows TagType::ALL entries
// On selection, stores selected_tag

unsafe fn select_image(&mut self) {
    let tag = self.selected_tag.unwrap();
    // ... file browser ...
    match image::load_bmp(path, tag.image_format) {
        Ok(img) => { self.image_data = Some(img); ... }
        Err(_) => { ... }
    }
}

unsafe fn write_to_tag(&mut self) {
    let tag = self.selected_tag.unwrap();
    let img = self.image_data.as_ref().unwrap();

    match (tag.protocol, img) {
        (Protocol::IsodepBwry, AnyImage::Bwry(image)) => {
            let mut proto = protocol_bwry::BwryProtocol::new();
            proto.write_image(image)
        }
        (Protocol::IsodepGenb, AnyImage::Bwr(image)) => {
            let mut proto = protocol_genb::GenbProtocol::new();
            proto.write_image(image)
        }
        _ => unreachable!("Tag type and image format mismatch"),
    }
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/tag_type.rs` | **Create** | TagType, ImageFormat, Protocol enums (~60 lines) |
| `src/protocol_common.rs` | **Create** | Extract shared NFC helpers from protocol.rs (~150 lines) |
| `src/protocol_bwry.rs` | **Create** | Renamed/refactored from protocol.rs (~350 lines) |
| `src/protocol_genb.rs` | **Create** | New GenB state machine (~400 lines) |
| `src/image.rs` | **Modify** | Add generics, Image<F>, AnyImage, BWR encoding (~150 lines added) |
| `src/main.rs` | **Modify** | Add tag selection menu, dispatch logic (~80 lines added) |
| `src/protocol.rs` | **Delete** | Split into protocol_common/protocol_bwry |
| `scripts/convert_to_bmp.py` | **Modify** | Add --format bwr\|bwry flag (~40 lines) |

## Implementation Order

1. **Create `src/tag_type.rs`** - Foundation types
2. **Create `src/protocol_common.rs`** - Extract shared NFC code
3. **Create `src/protocol_bwry.rs`** - Refactor existing protocol
4. **Modify `src/image.rs`** - Add generics and BWR encoding
5. **Update `src/main.rs`** - Add tag selection, update imports
6. **Delete `src/protocol.rs`** - Now split into modules
7. **Build and test BWRY regression**
8. **Create `src/protocol_genb.rs`** - New GenB protocol
9. **Update `scripts/convert_to_bmp.py`** - Add BWR support
10. **Full testing**

## Key Protocol Differences Reference

| Aspect | BWRY (protocol_bwry) | GenB (protocol_genb) |
|--------|----------------------|----------------------|
| GPIO1 delay | 200ms | 50ms |
| Init sequence | DISPLAY_INIT, E0/E6/A5 regs | 8 register pairs (01-4F) |
| Data transfer | Single 10KB stream | Two 5KB streams (reg 0x24, 0x26) |
| Refresh | REFRESH cmd (74 02 15 02 00) | Write 0xF7 to reg 0x22, select 0x20 |
| Ready condition | status != 0x00 | status == 0x01 |
| Poll interval | 400ms | 200ms |
| Initial wait | 10s | 4s |
| Cleanup | reg 0x02, 0x07 | None |

## BWR Image Encoding

```rust
// Dual 1-bit buffers, 8 pixels per byte, MSB first
// First 5000 bytes: B/W buffer (white=1, black=0)
// Second 5000 bytes: Red buffer (red=1, not-red=0)

for row in 0..200 {
    for x_byte in 0..25 {  // 200 pixels / 8 = 25 bytes per row
        let mut bw_byte = 0u8;
        let mut red_byte = 0u8;
        for bit in 0..8 {
            let (is_white, is_red) = map_to_bwr(pixel);
            bw_byte = (bw_byte << 1) | (is_white as u8);
            red_byte = (red_byte << 1) | (is_red as u8);
        }
        data[row * 25 + x_byte] = bw_byte;           // B/W buffer
        data[5000 + row * 25 + x_byte] = red_byte;   // Red buffer
    }
}
```

## Verification

1. **Build**: `cargo build --release`
2. **BWRY regression**: Write test image to BWRY tag, verify 4-color stripes
3. **GenB test**: Write BWR test image to BWR tag, verify 3-color stripes
4. **Type safety**: Try `proto_bwry.write_image(&bwr_image)` - should be compile error
5. **Python script**: Test `--format bwr` and `--format bwry` flags
