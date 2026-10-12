# verify

The differential harness compares geoprims with independent reference implementations. Its container pins and builds GeographicLib C++ 2.7, PROJ 9.9.0, H3 C 4.5.0, S2 C++ 0.13.1, NOAA WMM2025 C, and NGS HTDP 3.6.0 from their official source archives. Every archive is checked against its committed SHA-256 digest before it is built.

Build the container and run one known-answer smoke case through every reference:

```bash
docker build -f verify/Dockerfile -t geoprims-references .
docker run --rm geoprims-references
```

The container exits nonzero if a version or known answer differs. The release differential runner can use the same image for its 10,000-case family checks.
