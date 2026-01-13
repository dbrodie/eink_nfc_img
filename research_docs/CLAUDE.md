# E-Ink NFC Tag Reverse Engineering

## Background

This directory contains reverse engineering research of the official Android app (`DMPL0154FN1.1.apk`) for GoodDisplay/Waveshare NFC e-ink tags. The app was decompiled using JADX to understand the NFC communication protocols used to write images to these displays.

The research revealed that a single APK supports 10 different e-ink display models with varying sizes, resolutions, and color capabilities. Each display type uses specific NFC protocols depending on the hardware.

## Scope

### Covered

- **10 display models** supported by the DMPL0154FN1.1.apk
- **IsoDep protocols** (ISO 14443-4) for all displays
- **NfcA protocols** for displays that support it
- **Display sizes**: 1.54" to 7.5"
- **Color modes**: Black/White, BWR (3-color), BWRY (4-color)
- **Password authentication** protocol (optional feature)

### Not Covered

- **IsoDep_GenA protocol**: The `r()` method (8839 bytecode instructions) could not be fully decompiled. This affects displays at indexes 1-7 and 9.
- **NfcA protocols**: `NfcA_Gen` and `NfcA_154` are identified but not documented.
- **Dithering algorithms**: The Floyd-Steinberg dithering in `w0/a.java` is not documented.
- **Other APKs**: Only DMPL0154FN1.1.apk was analyzed.

## Documentation

### Research Summary

- [RESEARCH_SUMMARY.md](RESEARCH_SUMMARY.md) - Master index of all display models, protocols, and documentation status

### Protocol Documentation

| Protocol | File | Status |
|----------|------|--------|
| IsoDep_BWRY | [PROTOCOL_IsoDep_BWRY.md](PROTOCOL_IsoDep_BWRY.md) | Complete |
| IsoDep_GenB | [PROTOCOL_IsoDep_GenB.md](PROTOCOL_IsoDep_GenB.md) | Complete |
| Password | [PROTOCOL_Password.md](PROTOCOL_Password.md) | Complete |
| IsoDep_GenA | - | Not documented (decompilation failed) |
| NfcA_Gen | - | Not documented |
| NfcA_154 | - | Not documented |

## Source Files

| File | Description |
|------|-------------|
| `DMPL0154FN1.1.apk` | Official Android app |
| `decompiled/` | JADX decompiled output |

### Key Decompiled Classes

| File | Purpose |
|------|---------|
| `decompiled/sources/waveshare/feng/nfctag/activity/a.java` | NFC protocol handler - contains all NFC commands |
| `decompiled/sources/waveshare/feng/nfctag/activity/MainActivity.java` | Main UI, display configuration arrays, NFC orchestration |
| `decompiled/sources/w0/a.java` | 4-color Floyd-Steinberg dithering algorithm |
