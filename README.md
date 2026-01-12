# DMPL0154FN1 Flipper Zero App

A Flipper Zero application for writing images to the GoodDisplay DMPL0154FN1 4-color NFC e-ink tag.

## Features

- Write 4-color images (Black, White, Red, Yellow) to NFC e-ink tags
- Load images from SD card
- Simple menu-based UI

## Supported Hardware

- **Tag**: GoodDisplay DMPL0154FN1
- **Display**: 200×200 pixels, 4-color (BWRY)
- **Interface**: NFC IsoDep (ISO 14443-4)

## Building

### Prerequisites

1. Install Rust nightly:
   ```bash
   rustup toolchain install nightly
   rustup target add --toolchain nightly thumbv7em-none-eabihf
   ```

2. Clone flipperzero-rs (may be needed for linking):
   ```bash
   git clone https://github.com/flipperzero-rs/flipperzero-rs
   ```

### Build

```bash
cd flipper_app
cargo +nightly build --release
```

The output `.fap` file will be in `target/thumbv7em-none-eabihf/release/`.

### Install

1. Copy the `.fap` file to your Flipper Zero SD card under `/ext/apps/NFC/`
2. On Flipper: Navigate to Apps → NFC → DMPL0154FN1

## Image Format

The app uses `.4ei` files - a simple binary format:

```
Offset  Size  Description
0       4     Magic: "4EI1"
4       2     Width (little-endian, must be 200)
6       2     Height (little-endian, must be 200)
8       10000 Image data (2 bits per pixel)
```

### Pixel Encoding

| Color  | 2-bit value |
|--------|-------------|
| Black  | 00          |
| White  | 01          |
| Yellow | 10          |
| Red    | 11          |

4 pixels are packed per byte, MSB first:
```
Byte = (P0 << 6) | (P1 << 4) | (P2 << 2) | P3
```

### Image Conversion

Place `.4ei` files in `/ext/eink/` on your SD card.

A Python conversion script is available in the main project directory.

## Usage

1. Launch the app from Apps → NFC → DMPL0154FN1
2. Select "Select Image" and choose a `.4ei` file
3. Hold the DMPL0154FN1 tag near the Flipper Zero
4. Select "Write to Tag"
5. Keep the tag in place until "Success!" appears (~15-20 seconds)

## Protocol

See [PROTOCOL.md](../PROTOCOL.md) for the full NFC command reference.

## License

MIT

## Credits

- Protocol reverse-engineered from the official GoodDisplay Android app
- Based on [flipperzero-waveshare-nfc](https://github.com/mogenson/flipperzero-waveshare-nfc) by Mike Mogenson
- Uses [flipperzero-rs](https://github.com/flipperzero-rs/flipperzero-rs) Rust bindings
