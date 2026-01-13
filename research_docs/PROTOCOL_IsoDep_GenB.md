# IsoDep_GenB Protocol Documentation

## Overview

- **Protocol**: IsoDep_GenB
- **Display**: 1.54inch e-Paper B (Index 8)
- **Resolution**: 200×200 pixels
- **Colors**: Black, White, Red (BWR 3-color)
- **NFC Interface**: IsoDep (ISO 14443-4)
- **Source**: `decompiled/sources/waveshare/feng/nfctag/activity/a.java` method `b()`

---

## NFC Technology

- **Interface**: IsoDep (ISO 14443-4)
- **Timeout**: 1700ms configured in app
- **Max transceive length**: ~253 bytes (standard)

---

## Command Structure

### Request Format

```
[0x74] [CMD] [P1] [P2] [Lc] [Data...]
```

| Field | Description |
|-------|-------------|
| 0x74 | Command class byte (constant) |
| CMD | Command code |
| P1 | Parameter 1 |
| P2 | Parameter 2 |
| Lc | Length of data |
| Data | Command-specific data |

### Response Format

```
[Data...] [SW1] [SW2]
```

Status words are at the END of the response (standard ISO 7816-4 APDU format).
For commands with no data, response is just `[SW1] [SW2]`.
For READ_STATUS, response is `[STATUS] [SW1] [SW2]` where STATUS byte comes first.

| Response | Meaning |
|----------|---------|
| `90 00` | Success |

---

## Protocol Sequence

### 1. Initialization

```java
// Authentication/Init command
transceive({0x74, 0xB1, 0x00, 0x00, 0x08, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77});

// GPIO control
transceive({0x74, 0x97, 0x00, 0x08, 0x00});  // delay 50ms
transceive({0x74, 0x97, 0x01, 0x08, 0x00});  // delay 50ms
```

### 2. Display Configuration

```java
// Register 0x01 = 0xC7 0x00 0x01
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x01});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x03, 0xC7, 0x00, 0x01});

// Register 0x11 = 0x01
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x11});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x01});

// Register 0x44 = 0x00 0x18
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x44});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x02, 0x00, 0x18});

// Register 0x45 = 0xC7 0x00 0x00 0x00
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x45});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x04, 0xC7, 0x00, 0x00, 0x00});

// Register 0x3C = 0x05
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x3C});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x05});

// Register 0x18 = 0x80
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x18});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x80});

// Register 0x4E = 0x00
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x4E});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x00});

// Register 0x4F = 0xC7 0x00
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x4F});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x02, 0xC7, 0x00});  // delay 100ms
```

### 3. Send Black/White Data

```java
// Select register 0x24 (B/W buffer)
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x24});

// Send 5,000 bytes in 250-byte chunks (20 packets)
byte[] packet = new byte[260];
packet[0] = 0x74;
packet[1] = 0x9A;  // Write data command
packet[2] = 0x00;
packet[3] = 0x0E;
packet[4] = 0xFA;  // 250 bytes

for (int i = 0; i < 20; i++) {
    System.arraycopy(bwData, i * 250, packet, 5, 250);
    transceive(packet);
}
```

### 4. Send Red Data

```java
// Select register 0x26 (Red buffer)
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x26});

// Send 5,000 bytes in 250-byte chunks (20 packets)
for (int i = 0; i < 20; i++) {
    System.arraycopy(redData, i * 250, packet, 5, 250);
    transceive(packet);
}
```

### 5. Trigger Display Refresh

```java
// Select register 0x22
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x22});

// Write 0xF7 to trigger refresh
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0xF7});

// Select register 0x20
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x20});  // delay 4000ms

// Poll for completion (wait until response[0] == 0x01)
while (transceive({0x74, 0x9B, 0x00, 0x0F, 0x01})[0] != 0x01) {
    sleep(200);
}
```

---

## Command Reference

| Command | Bytes | Description |
|---------|-------|-------------|
| INIT | `74 B1 00 00 08 00 11 22 33 44 55 66 77` | Initialize NFC communication |
| GPIO_0 | `74 97 00 08 00` | GPIO/Power control |
| GPIO_1 | `74 97 01 08 00` | GPIO/Power control |
| SELECT_REG | `74 99 00 0D 01 XX` | Select register XX |
| WRITE_DATA | `74 9A 00 0E LEN [data]` | Write LEN bytes to selected register |
| READ_STATUS | `74 9B 00 0F 01` | Read busy status |

---

## Register Map

| Register | Value | Description |
|----------|-------|-------------|
| 0x01 | 0xC7 0x00 0x01 | Driver output control |
| 0x11 | 0x01 | Data entry mode |
| 0x44 | 0x00 0x18 | RAM X address range |
| 0x45 | 0xC7 0x00 0x00 0x00 | RAM Y address range |
| 0x3C | 0x05 | Border waveform |
| 0x18 | 0x80 | Temperature sensor |
| 0x4E | 0x00 | RAM X address counter |
| 0x4F | 0xC7 0x00 | RAM Y address counter |
| 0x24 | [data] | B/W RAM (write image data) |
| 0x26 | [data] | Red RAM (write image data) |
| 0x22 | 0xF7 | Display update control (trigger refresh) |
| 0x20 | - | Master activation |

---

## Pixel Encoding (BWR 3-color)

### Dual Buffer System

BWR displays use two separate 1-bit buffers:
- **B/W Buffer (register 0x24)**: Black=0, White=1
- **Red Buffer (register 0x26)**: Not-Red=0, Red=1

### Data Size

- Resolution: 200×200 pixels
- Bits per pixel: 1 bit per buffer
- Bytes per buffer: 200 × 200 / 8 = 5,000 bytes
- Total data: 10,000 bytes (5,000 B/W + 5,000 Red)
- Chunk size: 250 bytes
- Total packets: 40 (20 B/W + 20 Red)

### Byte Packing

8 pixels packed per byte (MSB first):

```
Byte = [P0][P1][P2][P3][P4][P5][P6][P7]
     = (P0 << 7) | (P1 << 6) | ... | P7
```

### Encoding Logic

```java
for (int row = 0; row < height; row++) {
    for (int byteCol = 0; byteCol < width / 8; byteCol++) {
        byte bwByte = 0;
        byte redByte = 0;

        for (int bit = 0; bit < 8; bit++) {
            bwByte = (byte)(bwByte << 1);
            redByte = (byte)(redByte << 1);

            int pixelIndex = (byteCol * 8) + bit + (width * row);

            // B/W buffer: white pixels = 1
            if (originalPixel[pixelIndex] == WHITE) {
                bwByte |= 1;
            }

            // Red buffer: red pixels = 1 (inverted threshold)
            if ((redChannel[pixelIndex] & 0xFF) < 128) {
                redByte |= 1;
            }
        }

        bwData[(width / 8) * row + byteCol] = bwByte;
        redData[(width / 8) * row + byteCol] = redByte;
    }
}
```

### Color Mapping

| Original Color | B/W Buffer | Red Buffer | Display Result |
|----------------|------------|------------|----------------|
| White (0xFFFFFFFF) | 1 | 0 | White |
| Black (0xFF000000) | 0 | 0 | Black |
| Red (0xFFFF0000) | 0 | 1 | Red |

---

## Color Constants (Android)

| Color | ARGB Integer | Hex |
|-------|-------------|-----|
| Black | -16777216 | 0xFF000000 |
| White | -1 | 0xFFFFFFFF |
| Red | -65536 | 0xFFFF0000 |

---

## Timing

| Operation | Delay |
|-----------|-------|
| After GPIO_0 | 50ms |
| After GPIO_1 | 50ms |
| After reg 0x4F write | 100ms |
| After reg 0x20 select | 4000ms |
| Busy poll interval | 200ms |

---

## Data Sources

- Method: `b()` at `decompiled/sources/waveshare/feng/nfctag/activity/a.java:159-296`
- Display index: 8

---

*Reverse engineered from DMPL0154FN1.1.apk*
