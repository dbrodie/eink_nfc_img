# E-Ink NFC Flipper Zero App

Flipper Zero application for writing images to GoodDisplay NFC-powered e-ink tags.

## Project Overview

This is a `no_std` Rust application targeting the Flipper Zero. It communicates with e-ink NFC tags over IsoDep (ISO 14443-4) to transfer and display images.

The NFC protocols were reverse-engineered from the official Android app (`DMPL0154FN1.1.apk`). See `research_docs/` for detailed protocol documentation.

## Supported Displays

| Display | Colors | Protocol | Format |
|---------|--------|----------|--------|
| 1.54inch e-Paper Y | Black, White, Red, Yellow | IsoDep BWRY | 4-color |
| 1.54inch e-Paper B | Black, White, Red | IsoDep GenB | 3-color |

## Source Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Application entry point, GUI (ViewDispatcher, Submenu, Widget) |
| `src/tag_type.rs` | Tag type definitions (TagType, Protocol, ImageFormat enums) |
| `src/image.rs` | Image loading from SD card, generic Image<F> type |
| `src/protocol_common.rs` | Shared NFC primitives (commands, helpers) |
| `src/protocol_bwry.rs` | BWRY 4-color protocol state machine |
| `src/protocol_genb.rs` | GenB 3-color (BWR) protocol state machine |
| `scripts/convert_to_bmp.py` | Python script to convert images to compatible BMP format |

## Building

```bash
cargo build --release
```

Output: `target/thumbv7em-none-eabihf/release/eink_nfc_img.fap`

## Toolchain

- Rust nightly (`nightly-2025-08-31`)
- Target: `thumbv7em-none-eabihf` (ARM Cortex-M4)
- Uses `--relocatable` linker flag for FAP format

## Dependencies

- `flipperzero` v0.16.0 - High-level Flipper Zero bindings
- `flipperzero-sys` v0.16.0 - Low-level FFI bindings
- `flipperzero-rt` v0.16.0 - Runtime (entry point, linker script)
- `flipperzero-alloc` v0.16.0 - Global allocator for `alloc` crate

## Hardware

- **Tags**: GoodDisplay 1.54" e-ink NFC displays
- **Display Resolution**: 200x200 pixels
- **Interface**: NFC IsoDep (ISO 14443-4)
- **NFC IC**: FM1280

## Image Format (BMP)

The app loads standard 8-bit indexed BMP files. The palette depends on the target display:

### BWRY 4-color (e-Paper Y)

| Palette Index | Color |
|---------------|-------|
| 0 | Black (0, 0, 0) |
| 1 | White (255, 255, 255) |
| 2 | Yellow (255, 255, 0) |
| 3 | Red (255, 0, 0) |

### BWR 3-color (e-Paper B)

| Palette Index | Color |
|---------------|-------|
| 0 | Black (0, 0, 0) |
| 1 | White (255, 255, 255) |
| 2 | Red (255, 0, 0) |

Requirements:
- 200x200 pixels
- 8-bit indexed color (256 color palette)
- Uncompressed (BI_RGB)

These BMP files can be viewed in any standard image viewer.

### Converting Images

Use the provided Python script:

```bash
# Install dependencies
pip install pillow numpy

# Convert for BWRY display (4-color, default)
python scripts/convert_to_bmp.py input.png output.bmp --dither

# Convert for BWR display (3-color)
python scripts/convert_to_bmp.py input.png output.bmp --format bwr --dither

# Convert without dithering (for graphics with solid colors)
python scripts/convert_to_bmp.py input.png output.bmp

# Create test patterns
python scripts/convert_to_bmp.py test test_bwry.bmp
python scripts/convert_to_bmp.py test test_bwr.bmp --format bwr
```

Place `.bmp` files on the Flipper SD card under `/ext/`.

## NFC Protocol Summary

Both protocols use `0x74` prefix with APDU-like command structure.

### BWRY Protocol (4-color)
1. Initialize communication (`74 B1...`)
2. Configure display registers (E0, E6, A5)
3. Transfer 10,000 bytes of image data in 64-byte chunks
4. Trigger display refresh (`74 02 15 02 00`)
5. Poll busy status until complete (10s initial wait, 400ms poll)
6. Cleanup registers (02, 07)

### GenB Protocol (3-color BWR)
1. Initialize communication (`74 B1...`)
2. Configure 8 display registers (01, 11, 44, 45, 3C, 18, 4E, 4F)
3. Transfer 5,000 bytes B/W data to register 0x24
4. Transfer 5,000 bytes Red data to register 0x26
5. Trigger refresh (write 0xF7 to reg 0x22, select reg 0x20)
6. Poll busy status until complete (4s initial wait, 200ms poll)

See `research_docs/` for complete protocol documentation.
