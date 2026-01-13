# E-Ink NFC Writer for Flipper Zero

A Flipper Zero application for writing images to GoodDisplay/Waveshare NFC-powered e-ink displays.

## Features

- Write images to NFC e-ink tags via IsoDep (ISO 14443-4)
- Support for multiple display types and color modes
- Load standard BMP images from SD card
- Simple menu-based UI with tag type selection
- Floyd-Steinberg dithering for photo conversion

## Supported Tags

| Display | Resolution | Colors | Implemented | Tested |
|---------|------------|--------|:-----------:|:------:|
| 1.54inch e-Paper Y (DMPL0154FN1) | 200×200 | Black, White, Red, Yellow | ✅ | ✅ |
| 1.54inch e-Paper B | 200×200 | Black, White, Red | ✅ | ❌ |
| 2.13inch e-Paper | 250×122 | Black, White | ❌ | ❌ |
| 2.7inch e-Paper | 264×176 | Black, White | ❌ | ❌ |
| 2.9inch e-Paper | 296×128 | Black, White | ❌ | ❌ |
| 2.9inch e-Paper B | 296×128 | Black, White, Red | ❌ | ❌ |
| 4.2inch e-Paper | 400×300 | Black, White | ❌ | ❌ |
| 4.2inch e-Paper B | 400×300 | Black, White, Red | ❌ | ❌ |
| 7.5inch e-Paper | 800×480 | Black, White | ❌ | ❌ |
| 7.5inch HD e-Paper | 880×528 | Black, White | ❌ | ❌ |

**Have a tag that isn't implemented or tested?** Please open an issue and let us know! We'd love to work together to add support for your display. If you can capture NFC traffic or provide access to the hardware, that helps tremendously.

## Building

### Prerequisites

1. Install Rust nightly toolchain:
   ```bash
   rustup toolchain install nightly-2025-08-31
   rustup target add --toolchain nightly-2025-08-31 thumbv7em-none-eabihf
   ```

2. Install Python dependencies (for image conversion):
   ```bash
   pip install pillow numpy
   ```

### Build

```bash
cargo +nightly-2025-08-31 build --release
```

The output `.fap` file will be at `target/thumbv7em-none-eabihf/release/eink_nfc_img.fap`.

### Install on Flipper Zero

1. Copy the `.fap` file to your Flipper Zero SD card:
   ```
   /ext/apps/NFC/eink_nfc_img.fap
   ```

2. On Flipper: Navigate to **Apps → NFC → E-Ink NFC**

## Usage

### Converting Images

Use the provided Python script to convert images to the required BMP format:

```bash
# Convert for BWRY display (4-color, default)
python scripts/convert_to_bmp.py input.png output.bmp --dither

# Convert for BWR display (3-color)
python scripts/convert_to_bmp.py input.png output.bmp --format bwr --dither

# Without dithering (for graphics with solid colors)
python scripts/convert_to_bmp.py input.png output.bmp

# Create test patterns
python scripts/convert_to_bmp.py test test_bwry.bmp
python scripts/convert_to_bmp.py test test_bwr.bmp --format bwr
```

### Writing to a Tag

1. Copy your `.bmp` files to the Flipper Zero SD card (anywhere under `/ext/`)
2. Launch the app: **Apps → NFC → E-Ink NFC**
3. Select **"Select Image"**
4. Choose your tag type (e.g., "1.54inch e-Paper Y")
5. Browse and select your `.bmp` file
6. Select **"Write to Tag"**
7. Hold the e-ink tag against the Flipper Zero's NFC antenna
8. Wait for "Success!" message (~15-30 seconds depending on display)

**Tips:**
- Keep the tag steady against the Flipper during the entire write process
- The display will flicker during refresh - this is normal
- BWRY displays take longer (~20s) than BWR displays (~10s)

## Image Format

The app loads standard 8-bit indexed BMP files. Images are automatically matched to the selected tag type.

**Requirements:**
- 200×200 pixels (for 1.54" displays)
- 8-bit indexed color (256 color palette)
- Uncompressed (BI_RGB)

The conversion script handles resizing and palette conversion automatically.

## Protocol Documentation

The NFC protocols were reverse-engineered from the official GoodDisplay Android app. See the `research_docs/` directory for detailed protocol documentation:

- [PROTOCOL_IsoDep_BWRY.md](research_docs/PROTOCOL_IsoDep_BWRY.md) - 4-color protocol
- [PROTOCOL_IsoDep_GenB.md](research_docs/PROTOCOL_IsoDep_GenB.md) - 3-color BWR protocol
- [RESEARCH_SUMMARY.md](research_docs/RESEARCH_SUMMARY.md) - Overview of all display models

## License

MIT

## Credits

- Protocol reverse-engineered from the official GoodDisplay Android app (DMPL0154FN1.1.apk)
- Inspired by [flipperzero-waveshare-nfc](https://github.com/mogenson/flipperzero-waveshare-nfc) by Mike Mogenson
- Built with [flipperzero-rs](https://github.com/flipperzero-rs/flipperzero-rs) Rust bindings
