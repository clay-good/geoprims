# Embedded geomagnetic models

Byte-for-byte copies of the official coefficient files. Replace them only with a new official release, and update the digests.

| File | Asset id | Source | SHA-256 |
|---|---|---|---|
| `WMM2025.COF` | `wmm2025` | NCEI, [WMM2025COF.zip](https://www.ncei.noaa.gov/sites/default/files/2024-12/WMM2025COF.zip) (released 2024-11-13) | `dfa8597825af4e0b87ff4198a5b4fb661b3c49f4cd090cd0164e0259b075582f` |
| `igrf14coeffs.txt` | `igrf14` | IAGA V-MOD, [igrf14coeffs.txt](https://www.ngdc.noaa.gov/IAGA/vmod/coeffs/igrf14coeffs.txt) | `8f8d88403028fc4ee92c4f38d97b46e0a87e2cfc496045b43c9e26c1d6b0903c` |

The WMM2025 test values that verify the synthesis are in `core/crates/gp-geodesy/tests/data/WMM2025_TestValues.txt` (same archive). WMM and IGRF are works of the US government and IAGA and are free to redistribute.

`igrf14` is bundled in the geodesy module for now (the data-assets registry lists it as on-demand); it adds about 29 KB before compression.
