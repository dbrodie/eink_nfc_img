# E-Ink NFC Tag Research Summary

## Display Models

| Index | Display Name | Resolution | Colors | NFC Tech | Protocol |
|-------|--------------|------------|--------|----------|----------|
| 1 | 2.13inch e-Paper | 250×122 | B/W | IsoDep, NfcA | IsoDep_GenA, NfcA_Gen |
| 2 | 2.9inch e-Paper | 296×128 | B/W | IsoDep, NfcA | IsoDep_GenA, NfcA_Gen |
| 3 | 4.2inch e-Paper | 400×300 | B/W | IsoDep, NfcA | IsoDep_GenA, NfcA_Gen |
| 4 | 7.5inch e-Paper | 800×480 | B/W | IsoDep, NfcA | IsoDep_GenA, NfcA_Gen |
| 5 | 7.5inch HD e-Paper | 880×528 | B/W | IsoDep, NfcA | IsoDep_GenA, NfcA_Gen |
| 6 | 2.7inch e-Paper | 264×176 | B/W | IsoDep, NfcA | IsoDep_GenA, NfcA_Gen |
| 7 | 2.9inch e-Paper B | 296×128 | BWR | IsoDep, NfcA | IsoDep_GenA, NfcA_Gen |
| 8 | 1.54inch e-Paper B | 200×200 | BWR | IsoDep, NfcA | IsoDep_GenB, NfcA_154 |
| 9 | 4.2inch e-Paper B | 400×300 | BWR | IsoDep, NfcA | IsoDep_GenA, NfcA_Gen |
| 10 | 1.54inch e-Paper Y (DMPL0154FN1) | 200×200 | BWRY | IsoDep | IsoDep_BWRY |

## Protocol Reference

| Protocol Name | NFC Tech | Class | Method | Source File | Documentation |
|---------------|----------|-------|--------|-------------|---------------|
| IsoDep_GenA | IsoDep | `a` | `r()` | `decompiled/sources/waveshare/feng/nfctag/activity/a.java` | Not documented (decompilation failed) |
| IsoDep_GenB | IsoDep | `a` | `b()` | `decompiled/sources/waveshare/feng/nfctag/activity/a.java` | [PROTOCOL_IsoDep_GenB.md](PROTOCOL_IsoDep_GenB.md) |
| IsoDep_BWRY | IsoDep | `a` | `d()` | `decompiled/sources/waveshare/feng/nfctag/activity/a.java` | [PROTOCOL_IsoDep_BWRY.md](PROTOCOL_IsoDep_BWRY.md) |
| NfcA_Gen | NfcA | `a` | `s()` | `decompiled/sources/waveshare/feng/nfctag/activity/a.java` | Not documented |
| NfcA_154 | NfcA | `a` | `c()` | `decompiled/sources/waveshare/feng/nfctag/activity/a.java` | Not documented |

## Additional Documentation

| Document | Description |
|----------|-------------|
| [PROTOCOL_Password.md](PROTOCOL_Password.md) | Optional password authentication for IsoDep and NfcA |

## Data Sources

- Display names: `decompiled/resources/res/values/strings.xml`
- Dimensions: `decompiled/sources/waveshare/feng/nfctag/activity/MainActivity.java` arrays `f3994y0`, `f3996z0`
- Protocol routing: `decompiled/sources/waveshare/feng/nfctag/activity/a.java` method `v()` lines 1210-1228
