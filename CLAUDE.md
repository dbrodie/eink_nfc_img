# DMPL0154FN1 Flipper Zero App

Flipper Zero application for writing images to GoodDisplay DMPL0154FN1 4-color NFC e-ink tags.

## Project Overview

This is a `no_std` Rust application targeting the Flipper Zero. It communicates with DMPL0154FN1 e-ink tags over NFC IsoDep (ISO 14443-4) to transfer and display 4-color images.

The NFC protocol was reverse-engineered from the official Android app (`DMPL0154FN1.1.apk`). See `PROTOCOL.md` for the complete protocol documentation.

## Source Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Application entry point, GUI (ViewDispatcher, Submenu, Widget) |
| `src/protocol.rs` | NFC protocol implementation using callback-based poller API |
| `src/image.rs` | Image loading from SD card (8-bit indexed BMP format) |
| `scripts/convert_to_bmp.py` | Python script to convert images to compatible BMP format |

## Building

```bash
cargo build --release
```

Output: `target/thumbv7em-none-eabihf/release/dmpl0154fn1.fap`

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

- **Tag**: GoodDisplay DMPL0154FN1
- **Display**: 200x200 pixels, 4-color (Black, White, Red, Yellow)
- **Interface**: NFC IsoDep (ISO 14443-4)
- **NFC IC**: FM1280

## Image Format (BMP)

The app loads standard 8-bit indexed BMP files with a 4-color palette:

| Palette Index | Color |
|---------------|-------|
| 0 | Black (0, 0, 0) |
| 1 | White (255, 255, 255) |
| 2 | Yellow (255, 255, 0) |
| 3 | Red (255, 0, 0) |

Requirements:
- 200x200 pixels
- 8-bit indexed color (256 color palette)
- Uncompressed (BI_RGB)
- Only first 4 palette entries are used

These BMP files can be viewed in any standard image viewer.

### Converting Images

Use the provided Python script:

```bash
# Install dependencies
pip install pillow numpy

# Convert with dithering (recommended for photos)
python scripts/convert_to_bmp.py input.png output.bmp --dither

# Convert without dithering (for graphics with solid colors)
python scripts/convert_to_bmp.py input.png output.bmp

# Create a test pattern
python scripts/convert_to_bmp.py test test.bmp
```

Place `.bmp` files in `/ext/eink/` on the Flipper SD card.

## NFC Protocol Summary

Commands use `0x74` prefix with APDU-like structure. The protocol sequence:

1. Initialize communication (`74 B1...`)
2. Configure display registers (E0, E6, A5)
3. Transfer 10,000 bytes of image data in 250-byte chunks (`74 9A...`)
4. Trigger display refresh (`74 02 15 02 00`)
5. Poll busy status until complete
6. Cleanup registers

See `PROTOCOL.md` for complete command reference.
