# Geodesy test data

## TMcoords-sample.dat

1,000 points of `TMcoords.dat`, GeographicLib's exact transverse Mercator test set by C. F. F. Karney (80-digit arithmetic, k0 = 0.9996, WGS 84): the points within 3.5° of the central meridian and at or below 84° N, every 11th. Columns: lat, lon from the central meridian, x, y (m, no false origin), convergence (degrees), scale.

- Source: https://sourceforge.net/projects/geographiclib/files/testdata/TMcoords.dat.gz (retrieved 2026-09-19)
- SHA-256 of the downloaded `.gz`: `91fb3a046625426b77e388a371c0a08ec14e8d98614f13cbd91019dd8baa7721`
- Regenerate: `gunzip -k TMcoords.dat.gz && awk '$2<=3.5 && $1<=84' TMcoords.dat | awk 'NR%11==1' | head -1000 > TMcoords-sample.dat`
