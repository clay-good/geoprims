# Navigation test data

`GeodTest-sample.dat` is every 10th line (1,000 lines) of `GeodTest-short.dat`, GeographicLib's geodesic test set by C. F. F. Karney: geodesics on WGS 84 computed with high-precision arithmetic, accurate to 0.1 nm. Columns: lat1 lon1 azi1 lat2 lon2 azi2 s12 a12 m12 S12 (degrees, meters, m²).

- Source: https://sourceforge.net/projects/geographiclib/files/testdata/GeodTest-short.dat.gz (retrieved 2026-09-19)
- SHA-256 of the downloaded `.gz`: `31d376e8158f7af26277d887d7a1e7726e14d172db5a848a1730dd4886ab6121`
- Regenerate: `gunzip -k GeodTest-short.dat.gz && awk 'NR%10==1' GeodTest-short.dat > GeodTest-sample.dat`
- License: GeographicLib test data is MIT-licensed with GeographicLib.
