# Password Protocol Documentation

## Overview

- **Feature**: Optional password authentication for NFC e-ink tags
- **Purpose**: Protect tags from unauthorized writes
- **Source**: `decompiled/sources/waveshare/feng/nfctag/activity/a.java` methods `o()` and `p()`

This is an optional feature that can be checked before writing an image. If a tag has password protection enabled, the correct password must be sent before the tag will accept image data.

---

## IsoDep Password Protocol

### Method

`o(byte[] bArr)` at `a.java:718-746`

### Command Format

```
[0x74] [0xB3] [0x00] [0x00] [LEN+1] [LEN] [PASSWORD...]
```

| Field | Value | Description |
|-------|-------|-------------|
| 0x74 | Constant | Command class byte |
| 0xB3 | Constant | Password command |
| 0x00 | Constant | Parameter 1 |
| 0x00 | Constant | Parameter 2 |
| LEN+1 | Variable | Password length + 1 |
| LEN | Variable | Password length |
| PASSWORD | Variable | Password bytes |

### Response Format

```
[0x6A] [STATUS]
```

| Response | Return Value | Meaning |
|----------|--------------|---------|
| `6A 00` | 0 | No password set on tag |
| `6A 01` | 1 | Password correct |
| `6A 02` | 2 | Password incorrect |
| `6A 03` | 3 | Password enabled (need to authenticate) |

### Example

```java
// Check/send password "1234"
byte[] password = {0x31, 0x32, 0x33, 0x34};  // "1234" in ASCII
int pwdLen = 4;

byte[] cmd = new byte[6 + pwdLen];
cmd[0] = 0x74;
cmd[1] = 0xB3;
cmd[2] = 0x00;
cmd[3] = 0x00;
cmd[4] = (byte)(pwdLen + 1);
cmd[5] = (byte)pwdLen;
System.arraycopy(password, 0, cmd, 6, pwdLen);

byte[] response = isoDep.transceive(cmd);
// Check response[0] == 0x6A, response[1] for status
```

---

## NfcA Password Protocol

### Method

`p(byte[] bArr)` at `a.java:748-774`

### Step 1: Check if Password is Enabled

```
[0xFF] [0xFE] [0xFA]
```

| Response | Meaning |
|----------|---------|
| `FF 00` | No password set (return 0) |
| `FF EE` | Password enabled, proceed to step 2 |

### Step 2: Send Password

```
[0xFF] [0xFE] [0x00] [LEN] [PASSWORD...]
```

| Field | Value | Description |
|-------|-------|-------------|
| 0xFF | Constant | Command prefix |
| 0xFE | Constant | Password command |
| 0x00 | Constant | Send password sub-command |
| LEN | Variable | Password length |
| PASSWORD | Variable | Password bytes |

### Response

| Response | Return Value | Meaning |
|----------|--------------|---------|
| `FF 00` | 1 | Password correct |
| Other | 2 | Password incorrect |

### Example

```java
// Check if password is enabled
byte[] checkCmd = {0xFF, 0xFE, 0xFA};
byte[] response = nfcA.transceive(checkCmd);

if (response[0] == 0xFF && response[1] == 0x00) {
    // No password set
    return 0;
}

if (response[0] == 0xFF && response[1] == 0xEE) {
    // Password enabled, send password
    byte[] password = {0x31, 0x32, 0x33, 0x34};  // "1234"
    int pwdLen = 4;

    byte[] sendCmd = new byte[4 + pwdLen];
    sendCmd[0] = 0xFF;
    sendCmd[1] = 0xFE;
    sendCmd[2] = 0x00;
    sendCmd[3] = (byte)pwdLen;
    System.arraycopy(password, 0, sendCmd, 4, pwdLen);

    byte[] response2 = nfcA.transceive(sendCmd);
    if (response2[0] == 0xFF && response2[1] == 0x00) {
        return 1;  // Password correct
    }
    return 2;  // Password incorrect
}
```

---

## Usage in App

The app checks password status when a tag is detected, before allowing image writes. If password is enabled and incorrect/not provided, the write operation will fail.

---

## Data Sources

- IsoDep method: `o()` at `decompiled/sources/waveshare/feng/nfctag/activity/a.java:718-746`
- NfcA method: `p()` at `decompiled/sources/waveshare/feng/nfctag/activity/a.java:748-774`

---

*Reverse engineered from DMPL0154FN1.1.apk*
