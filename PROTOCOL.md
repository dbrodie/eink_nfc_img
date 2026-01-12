# DMPL0154FN1 NFC Protocol Documentation

## Overview

This document describes the NFC communication protocol for the GoodDisplay DMPL0154FN1 4-color e-ink tag, reverse-engineered from the official Android app (DMPL0154FN1.1.apk).

The DMPL0154FN1 uses the **IsoDep** (ISO-DEP / ISO 14443-4) NFC interface. The app also supports NfcA for some displays, but the 4-color 1.54" display specifically uses IsoDep.

---

## Key Findings

### Display Type Identification

The app supports multiple display sizes. The DMPL0154FN1 corresponds to **index 10** in the internal display arrays:

| Index | Width | Height | Type | Description |
|-------|-------|--------|------|-------------|
| 8 | 200 | 200 | BWR | 1.54" 3-color (Black/White/Red) |
| **10** | **200** | **200** | **BWRY** | **1.54" 4-color (Black/White/Red/Yellow) - DMPL0154FN1** |

### NFC Technology

- **Interface**: IsoDep (ISO 14443-4)
- **Timeout**: 1700ms configured in app
- **Max transceive length**: Queried but standard (~253 bytes)

---

## Command Structure

### General Format

Commands follow this structure:

```
[0x74] [CMD] [P1] [P2] [Lc] [Data...]
```

Where:
- `0x74` - Command class byte (constant)
- `CMD` - Command code
- `P1`, `P2` - Parameters
- `Lc` - Length of data
- `Data` - Command-specific data

### Response Format

Responses have status bytes at the beginning:

```
[SW1] [SW2] [Data...]
```

Where:
- `0x90 0x00` - Success
- `0x6A 0xXX` - Error/Status codes (for password commands)

---

## Protocol Sequence for 4-Color Display

The complete sequence to write an image to the DMPL0154FN1:

### 1. Initialization

```java
// Connect and set timeout
isoDep.connect();
isoDep.setTimeout(1700);

// Authentication/Init command
transceive({0x74, 0xB1, 0x00, 0x00, 0x08, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77});
// Expected response: 0x90 0x00

// Power/GPIO control commands
transceive({0x74, 0x97, 0x00, 0x08, 0x00});  // delay 50ms
transceive({0x74, 0x97, 0x01, 0x08, 0x00});  // delay 200ms
```

### 2. Display Configuration

```java
// Display init
transceive({0x74, 0x00, 0x15, 0x00, 0x00});  // delay 100ms

// Write register 0xE0 = 0x02
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0xE0});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x02});

// Write register 0xE6 = 0x5D
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0xE6});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x5D});

// Write register 0xA5 = 0x00
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0xA5});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x00});  // delay 100ms
```

### 3. Start Data Transfer

```java
// Begin data transfer mode
transceive({0x74, 0x01, 0x15, 0x01, 0x00});
```

### 4. Send Image Data

```java
// Prepare 10,000 bytes of image data (200×200×2bpp / 8 = 10,000)
byte[] imageData = new byte[10000];
// ... populate with 4-color encoded pixels ...

// Send in 250-byte chunks (40 total packets)
byte[] packet = new byte[255];
packet[0] = 0x74;
packet[1] = 0x9A;  // Write data command
packet[2] = 0x00;
packet[3] = 0x0E;
packet[4] = 0xFA;  // 250 bytes

for (int i = 0; i < 10000; i += 250) {
    System.arraycopy(imageData, i, packet, 5, 250);
    transceive(packet);
}
```

### 5. Trigger Display Refresh

```java
// Trigger refresh
transceive({0x74, 0x02, 0x15, 0x02, 0x00});  // delay 10000ms

// Poll for completion
while (transceive({0x74, 0x9B, 0x00, 0x0F, 0x01})[0] == 0x00) {
    sleep(400);
}

// Final cleanup commands
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x02});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x00});  // delay 200ms
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x07});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0xA5});
```

---

## Command Reference

| Command | Bytes | Description |
|---------|-------|-------------|
| INIT | `74 B1 00 00 08 00 11 22 33 44 55 66 77` | Initialize NFC communication |
| GPIO_0 | `74 97 00 08 00` | GPIO/Power control |
| GPIO_1 | `74 97 01 08 00` | GPIO/Power control |
| DISPLAY_INIT | `74 00 15 00 00` | Initialize display |
| SELECT_REG | `74 99 00 0D 01 XX` | Select register XX |
| WRITE_DATA | `74 9A 00 0E LEN [data]` | Write LEN bytes |
| READ_STATUS | `74 9B 00 0F 01` | Read busy status |
| START_TX | `74 01 15 01 00` | Start data transmission |
| REFRESH | `74 02 15 02 00` | Trigger display refresh |
| PASSWORD | `74 B3 00 00 LEN+1 LEN [pwd]` | Password authentication |

---

## 4-Color Pixel Encoding

### Bit Layout

Each pixel is encoded as 2 bits:

| Color | Binary | Hex |
|-------|--------|-----|
| Black | 00 | 0x0 |
| White | 01 | 0x1 |
| Yellow | 10 | 0x2 |
| Red | 11 | 0x3 |

### Byte Packing

4 pixels are packed into each byte (MSB first):

```
Byte = [Pixel0][Pixel1][Pixel2][Pixel3]
     = (P0 << 6) | (P1 << 4) | (P2 << 2) | P3
```

Example: Black, White, Yellow, Red = `0b00_01_10_11` = `0x1B`

### Data Layout

The image data is organized as:
- 200 rows × 50 bytes per row = 10,000 bytes
- Row-major order
- Pixels packed left-to-right within each byte

The conversion from bitmap coordinates:

```java
for (int y = 0; y < 200; y++) {
    for (int x = 0; x < 50; x++) {
        byte b = 0;
        for (int bit = 0; bit < 4; bit++) {
            b = (byte)(b << 2);
            int pixel = bitmap[(x*4 + bit) * 200 + y];  // Column-first access
            if (pixel == WHITE) b |= 0x01;
            else if (pixel == YELLOW) b |= 0x02;
            else if (pixel == RED) b |= 0x03;
            // BLACK = 0x00 (default)
        }
        data[y * 50 + x] = b;
    }
}
```

Note: The actual code accesses pixels in column-first order with rotation.

---

## Color Constants (Android)

The app uses Android ARGB color integers:

| Color | ARGB Integer | Hex |
|-------|-------------|-----|
| Black | -16777216 | 0xFF000000 |
| White | -1 | 0xFFFFFFFF |
| Red | -65536 | 0xFFFF0000 |
| Yellow | -256 | 0xFFFFFF00 |

---

## Dithering

The app uses Floyd-Steinberg error diffusion dithering (modified Atkinson variant) to convert full-color images to 4-color:

### Palette Selection
```java
int[][] palette = {
    {BLACK, WHITE},           // Monochrome
    {BLACK, WHITE, RED},      // 3-color BWR
    {BLACK, WHITE, RED, YELLOW}  // 4-color BWRY (index 2+)
};
```

### Algorithm

1. For each pixel, find nearest color in palette using RGB Euclidean distance
2. Calculate error (difference between original and chosen color)
3. Distribute error to neighboring pixels:
   - Right (+1,0): 1/8
   - Right+1 (+2,0): 1/8
   - Down-left (-1,+1): 1/8
   - Down (0,+1): 1/8
   - Down-right (+1,+1): 1/8
   - Down+2 (0,+2): 1/8

This is a modified error diffusion that spreads error to 6 neighbors instead of the traditional 4.

---

## Password Protocol

### Check/Send Password (IsoDep)

```java
// Command format
byte[] cmd = {0x74, 0xB3, 0x00, 0x00, (byte)(pwdLen+1), (byte)pwdLen, ...password...};
byte[] response = transceive(cmd);
```

### Response Codes

| Response | Meaning |
|----------|---------|
| `6A 00` | No password set |
| `6A 01` | Password correct |
| `6A 02` | Password incorrect |
| `6A 03` | Password enabled |

---

## Source Files Reference

Key decompiled classes:

| File | Purpose |
|------|---------|
| `waveshare/feng/nfctag/activity/a.java` | NFC protocol handler |
| `waveshare/feng/nfctag/activity/MainActivity.java` | Main UI and NFC orchestration |
| `w0/a.java` | 4-color Floyd-Steinberg dithering |
| `com/askjeffreyliu/floydsteinbergdithering/Utils.java` | B/W dithering |

---

## Notes

1. The protocol appears to be based on a proprietary protocol for NFC e-ink displays
2. The 0x74 prefix and command structure suggests ISO 7816-4 APDU-like commands
3. Register addresses (0xE0, 0xE6, 0xA5, etc.) likely correspond to the e-paper driver IC
4. The ~12-16 second refresh time is handled by polling the busy status

---

*Reverse engineered from DMPL0154FN1.1.apk version - January 2026*
