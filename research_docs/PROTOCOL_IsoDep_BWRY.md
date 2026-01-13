# IsoDep_BWRY Protocol Documentation

## Overview

- **Protocol**: IsoDep_BWRY
- **Display**: 1.54inch e-Paper Y (Index 10, DMPL0154FN1)
- **Resolution**: 200×200 pixels
- **Colors**: Black, White, Red, Yellow (BWRY 4-color)
- **NFC Interface**: IsoDep (ISO 14443-4)
- **Source**: `decompiled/sources/waveshare/feng/nfctag/activity/a.java` method `d()`

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
[SW1] [SW2] [Data...]
```

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
transceive({0x74, 0x97, 0x01, 0x08, 0x00});  // delay 200ms
```

### 2. Display Configuration

```java
// Display init
transceive({0x74, 0x00, 0x15, 0x00, 0x00});  // delay 100ms

// Register 0xE0 = 0x02
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0xE0});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x02});

// Register 0xE6 = 0x5D
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0xE6});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x5D});

// Register 0xA5 = 0x00
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
// Send 10,000 bytes in 250-byte chunks (40 packets)
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

// Poll for completion (wait until response[0] != 0x00)
while (transceive({0x74, 0x9B, 0x00, 0x0F, 0x01})[0] == 0x00) {
    sleep(400);
}

// Cleanup: Register 0x02 = 0x00
transceive({0x74, 0x99, 0x00, 0x0D, 0x01, 0x02});
transceive({0x74, 0x9A, 0x00, 0x0E, 0x01, 0x00});  // delay 200ms

// Cleanup: Register 0x07 = 0xA5
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
| WRITE_DATA | `74 9A 00 0E LEN [data]` | Write LEN bytes to selected register |
| READ_STATUS | `74 9B 00 0F 01` | Read busy status |
| START_TX | `74 01 15 01 00` | Start data transmission |
| REFRESH | `74 02 15 02 00` | Trigger display refresh |

---

## Register Map

| Register | Value | Description |
|----------|-------|-------------|
| 0xE0 | 0x02 | Display configuration |
| 0xE6 | 0x5D | Display configuration |
| 0xA5 | 0x00 | Display configuration |
| 0x02 | 0x00 | Cleanup (after refresh) |
| 0x07 | 0xA5 | Cleanup (after refresh) |

---

## Pixel Encoding (BWRY 4-color)

### Single Buffer System

BWRY displays use a single buffer with 2 bits per pixel.

### Data Size

- Resolution: 200×200 pixels
- Bits per pixel: 2
- Bytes total: 200 × 200 × 2 / 8 = 10,000 bytes
- Chunk size: 250 bytes
- Total packets: 40

### Bit Layout

| Color | Binary | Hex |
|-------|--------|-----|
| Black | 00 | 0x0 |
| White | 01 | 0x1 |
| Yellow | 10 | 0x2 |
| Red | 11 | 0x3 |

### Byte Packing

4 pixels packed per byte (MSB first):

```
Byte = [Pixel0][Pixel1][Pixel2][Pixel3]
     = (P0 << 6) | (P1 << 4) | (P2 << 2) | P3
```

Example: Black, White, Yellow, Red = `0b00_01_10_11` = `0x1B`

### Encoding Logic

```java
for (int row = 0; row < 200; row++) {
    for (int byteCol = 0; byteCol < 50; byteCol++) {
        byte b = 0;
        for (int bit = 0; bit < 4; bit++) {
            b = (byte)(b << 2);
            int pixel = bitmap[((byteCol * 4) + bit) * 200 + row];  // Column-first access
            if (pixel == WHITE) b |= 0x01;
            else if (pixel == YELLOW) b |= 0x02;
            else if (pixel == RED) b |= 0x03;
            // BLACK = 0x00 (default)
        }
        data[row * 50 + byteCol] = b;
    }
}
```

Note: The code accesses pixels in column-first order (90-degree rotation).

### Color Mapping

| Original Color | 2-bit Value | Display Result |
|----------------|-------------|----------------|
| Black (0xFF000000) | 00 | Black |
| White (0xFFFFFFFF) | 01 | White |
| Yellow (0xFFFFFF00) | 10 | Yellow |
| Red (0xFFFF0000) | 11 | Red |

---

## Color Constants (Android)

| Color | ARGB Integer | Hex |
|-------|-------------|-----|
| Black | -16777216 | 0xFF000000 |
| White | -1 | 0xFFFFFFFF |
| Yellow | -256 | 0xFFFFFF00 |
| Red | -65536 | 0xFFFF0000 |

---

## Timing

| Operation | Delay |
|-----------|-------|
| After GPIO_0 | 50ms |
| After GPIO_1 | 200ms |
| After DISPLAY_INIT | 100ms |
| After reg 0xA5 write | 100ms |
| After REFRESH | 10000ms |
| Busy poll interval | 400ms |
| After cleanup reg 0x02 | 200ms |

---

## Data Sources

- Method: `d()` at `decompiled/sources/waveshare/feng/nfctag/activity/a.java:391-515`
- Display index: 10

---

*Reverse engineered from DMPL0154FN1.1.apk*
