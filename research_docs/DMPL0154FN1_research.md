# DMPL0154FN1 NFC E-Ink Tag Research

## Overview

The DMPL0154FN1 is a 4-color NFC-powered e-ink tag manufactured by GoodDisplay. This document summarizes research for reverse engineering the NFC protocol.

---

## Technical Specifications

| Specification | Value |
|---------------|-------|
| Model | DMPL0154FN1 |
| Manufacturer | GoodDisplay |
| Display Size | 1.54 inch |
| Resolution | 200 × 200 pixels |
| Colors | 4-color (Black, White, Red, Yellow) |
| Power Source | NFC-powered (battery-free, energy harvesting) |
| Refresh Time | ~12-16 seconds |
| Operating Temperature | 0°C to 40°C |
| Operating Humidity | 40% to 70% |
| Waterproof Rating | IP65 |
| Dimensions | 50 × 37 × 5.3 mm |
| Weight | ~13g |
| Form Factor | Complete tag with ABS case, corner holes for keychain/bag attachment |

### Internal Components

| Component | Model/Description |
|-----------|-------------------|
| Display Panel | GDEM0154F51H (4-color e-paper) |
| NFC Driver Board | DELM-0154S1 |
| NFC IC | FM1280 (ISO14443-A compliant) |

---

## NFC Protocol Information

### Confirmed Protocol Details (Reverse Engineered)

- **NFC Interface**: IsoDep (ISO 14443-4 / ISO-DEP)
- **Power Delivery**: NFC energy harvesting
- **Command Format**: Proprietary APDU-like structure with 0x74 prefix
- **Timeout**: 1700ms configured by app

### Command Structure

```
[0x74] [CMD] [P1] [P2] [Lc] [Data...]
Response: [SW1] [SW2] [Data...]
Success: 0x90 0x00
```

### Key Commands

| Command | Hex | Purpose |
|---------|-----|---------|
| INIT | `74 B1 00 00 08 00 11 22 33 44 55 66 77` | Initialize communication |
| SELECT_REG | `74 99 00 0D 01 XX` | Select e-paper register XX |
| WRITE_DATA | `74 9A 00 0E LEN [data]` | Write data to register |
| READ_STATUS | `74 9B 00 0F 01` | Check busy status |
| DISPLAY_INIT | `74 00 15 00 00` | Initialize display |
| START_TX | `74 01 15 01 00` | Begin image transfer |
| REFRESH | `74 02 15 02 00` | Trigger display refresh |
| PASSWORD | `74 B3 00 00 LEN+1 LEN [pwd]` | Password auth |

### FM1280 NFC IC

The FM1280 is the NFC interface chip. Key characteristics:
- ISO14443-A compliant
- Supports IsoDep (ISO 14443-4) protocol
- Energy harvesting capability for battery-free operation

### Similar Chips (for reference)

- **Chivotech TN2115S2**: Used in Waveshare NFC e-paper displays, 8 MHz Cortex-M0 MCU with NFC and energy harvesting (up to 300mW). Has been reverse engineered by Aaron Christophel.
- **ST25DV**: Common NFC dynamic tag IC with I²C interface, used in some e-paper implementations.

---

## Image Data Format

### Display Requirements

- **Resolution**: 200 × 200 pixels
- **Color Depth**: 4 colors (2 bits per pixel)
- **Total Data Size**: 200 × 200 × 2 bits = 80,000 bits = 10,000 bytes (uncompressed)

### 4-Color Pixel Encoding (Confirmed)

| Color | Binary | Hex | Android ARGB |
|-------|--------|-----|--------------|
| Black | 00 | 0x0 | 0xFF000000 |
| White | 01 | 0x1 | 0xFFFFFFFF |
| Yellow | 10 | 0x2 | 0xFFFFFF00 |
| Red | 11 | 0x3 | 0xFFFF0000 |

### Byte Packing

4 pixels per byte, MSB first:
```
Byte = (P0 << 6) | (P1 << 4) | (P2 << 2) | P3
Example: Black, White, Yellow, Red = 0b00_01_10_11 = 0x1B
```

### Data Transfer

- 10,000 bytes sent in 250-byte chunks (40 packets)
- Row-major order with column-first pixel access
- Transmitted via `0x74 0x9A 0x00 0x0E 0xFA [250 bytes]` command

### Supported Input Formats (via official app)

- BMP
- PNG
- JPG

### Image Conversion Pipeline

1. Resize/crop to 200×200 pixels
2. Apply Floyd-Steinberg error diffusion dithering to 4-color palette
3. Pack pixels into 2-bit format
4. Transmit via NFC in 250-byte chunks

---

## Comparison with Similar Tags

### GDEY0154D67 (Supported by Flipper nfc_eink)

| Feature | GDEY0154D67 | DMPL0154FN1 |
|---------|-------------|-------------|
| Size | 1.54 inch | 1.54 inch |
| Resolution | 200×200 | 200×200 |
| Colors | Monochrome (B/W) | 4-color (B/W/R/Y) |
| Interface | SPI | NFC |
| Driver IC | SSD1681 | DELM-0154S1 |
| Form Factor | Raw display panel | Complete NFC tag |

Despite same resolution, these are fundamentally different products with different protocols.

### Waveshare NFC E-Paper (Reverse Engineered)

Waveshare NFC e-paper displays have been reverse engineered by Aaron Christophel:
- Uses Chivotech TN2115S2 SoC
- Protocol has been documented
- Custom firmware available

The DMPL0154FN1 likely uses a different MCU/protocol but the reverse engineering approach would be similar.

---

## Official Software

### Android App

- **Name**: DMPL0154FN1.1 Mobile App
- **Size**: 14.7 MB
- **Download**: https://www.good-display.com/companyfile/29/
- **Last Updated**: 2025-12-08

### iOS App

- **Name**: NFC E-Tag
- **Requirements**: iOS 13.0+
- **Available on**: Apple App Store

---

## Flipper Zero nfc_eink App

### Current Status

The [nfc_eink app](https://github.com/RebornedBrain/nfc_eink) by RebornedBrain **does NOT support DMPL0154FN1**.

### Supported Displays (for reference)

**Waveshare (monochrome):**
- 2.13", 2.7", 2.9", 4.2", 7.5"

**GoodDisplay (monochrome):**
- GDEY0154D67, GDEY0213B74, GDEY029T94, GDEY037T03

---

## Related Reverse Engineering Projects

### Waveshare NFC E-Paper Custom Firmware
- **Author**: Aaron Christophel (atc1441)
- **GitHub**: https://github.com/atc1441/Waveshare_NFC_E-Paper_Display_custom_firmware
- **Approach**: Firmware dump → Ghidra disassembly → Protocol understanding → Custom firmware

### E-Paper Price Tags
- **Author**: Aaron Christophel (atc1441)
- **GitHub**: https://github.com/atc1441/E-Paper_Pricetags
- Custom BLE firmware for electronic shelf labels

### NFC E-Paper Writer (Android)
- **GitHub**: https://github.com/DevPika/nfc-epaper-writer-update
- Fork with color dithering and 3-color support
- Reference for NFC communication patterns

### Flipper Zero nfc_eink
- **GitHub**: https://github.com/RebornedBrain/nfc_eink
- Reference for Flipper Zero NFC e-ink implementation

---

## Web Sources

### Official Documentation
- **Product Page**: https://www.good-display.com/product/547.html
- **App Downloads**: https://www.good-display.com/companyfile/29/
- **GoodDisplay NFC Tags**: https://www.good-display.com/product/141/

### AliExpress Listing
- https://he.aliexpress.com/item/1005008032220781.html

### Flipper Zero Resources
- **nfc_eink App**: https://lab.flipper.net/apps/nfc_eink
- **nfc_eink GitHub**: https://github.com/RebornedBrain/nfc_eink
- **Flipper NFC Docs**: https://docs.flipper.net/zero/nfc

### Reverse Engineering References
- **Hackaday - Waveshare NFC E-Paper Hacking**: https://hackaday.com/2023/12/18/hacking-an-nfc-e-paper-display-from-waveshare-with-mystery-mcu/
- **Aaron Christophel YouTube**: https://www.youtube.com/watch?v=wsSWYC06b_U
- **Aaron Christophel GitHub**: https://github.com/atc1441

### E-Ink Image Conversion
- **Adafruit E-Ink Graphics Guide**: https://learn.adafruit.com/preparing-graphics-for-e-ink-displays
- **Waveshare Floyd-Steinberg**: https://www.waveshare.com/wiki/E-Paper_Floyd-Steinberg
- **Online Dithering Tool**: https://ditherit.com/

### Related Hardware Documentation
- **Waveshare NFC E-Paper Wiki**: https://www.waveshare.com/wiki/1.54inch_NFC-Powered_e-Paper_(BB)

---

*Last Updated: 2026-01-11*
