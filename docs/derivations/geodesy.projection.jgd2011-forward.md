<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to Japan plane rectangular (`geodesy.projection.jgd2011-forward`)

## Method

Japan's public surveys, cadastral maps, and municipal datasets are drawn on nineteen plane rectangular systems (平面直角座標系). They are defined by Ministry of Land, Infrastructure, Transport and Tourism Notice No. 9 of 2002, last amended in 2010, which the Geospatial Information Authority of Japan (GSI) publishes. The notice gives each system's origin and the prefectures it applies to. Its remarks fix the rest: a Gauss conformal projection, the X axis along the origin's meridian and positive to true north, the Y axis positive to true east, a scale factor of 0.9999 on the X axis, and X = 0.000 m, Y = 0.000 m at the origin. EPSG registers the nineteen systems on JGD2011 as 6669 to 6687.

So the projection is EPSG method 9807, transverse Mercator, with the system's origin latitude, its central meridian, `k₀ = 0.9999`, and no false easting or northing. The math is Krüger's series to sixth order in Karney's arrangement, the same series the generic transverse Mercator tool uses. This tool is that one with the notice's parameters fixed and named.

Three things make it a tool of its own:

| | What the tool does |
|---|---|
| The system | It is asked for. The notice assigns systems by prefecture and by island group, not by a band of longitude: Tokyo's mainland is IX, its Ogasawara islands XIV, XVIII, and XIX. |
| The axes | X is the northing and Y the easting, the reverse of most grids' order. The output names both. |
| The signs | The origin is at zero, so coordinates south or west of it are negative. |

Beside X and Y the tool returns the **grid convergence**, the angle from true north to grid north, and the **point scale factor**, grid distance over ground distance there.

The systems by prefecture, as the notice lists them:

| System | Origin | Applies to |
|---|---|---|
| I | 33°N 129°30′E | Nagasaki; Kagoshima's islands between 27°N and 32°N west of 130°E (Amami to 130°13′E) |
| II | 33°N 131°E | Fukuoka, Saga, Kumamoto, Oita, Miyazaki, the rest of Kagoshima |
| III | 36°N 132°10′E | Yamaguchi, Shimane, Hiroshima |
| IV | 33°N 133°30′E | Kagawa, Ehime, Tokushima, Kochi |
| V | 36°N 134°20′E | Hyogo, Tottori, Okayama |
| VI | 36°N 136°E | Kyoto, Osaka, Fukui, Shiga, Mie, Nara, Wakayama |
| VII | 36°N 137°10′E | Ishikawa, Toyama, Gifu, Aichi |
| VIII | 36°N 138°30′E | Niigata, Nagano, Yamanashi, Shizuoka |
| IX | 36°N 139°50′E | Tokyo (mainland), Fukushima, Tochigi, Ibaraki, Saitama, Chiba, Gunma, Kanagawa |
| X | 40°N 140°50′E | Aomori, Akita, Yamagata, Iwate, Miyagi |
| XI | 44°N 140°15′E | Hokkaido: the cities of Otaru, Hakodate, Date, and Hokuto; the Shiribeshi, Oshima, and Hiyama subprefectures; three towns of Iburi |
| XII | 44°N 142°15′E | The rest of Hokkaido |
| XIII | 44°N 144°15′E | Hokkaido: the cities of Kitami, Obihiro, Kushiro, Abashiri, and Nemuro; the Tokachi, Kushiro, and Nemuro subprefectures; nine towns of Okhotsk |
| XIV | 26°N 142°E | Tokyo south of 28°N, from 140°30′E to 143°E |
| XV | 26°N 127°30′E | Okinawa from 126°E to 130°E |
| XVI | 26°N 124°E | Okinawa west of 126°E |
| XVII | 26°N 131°E | Okinawa east of 130°E |
| XVIII | 20°N 136°E | Tokyo south of 28°N, west of 140°30′E |
| XIX | 26°N 154°E | Tokyo south of 28°N, east of 143°E |

## Equations

- `Y = k₀·B₁·a·η`, `X = k₀·B₁·a·ξ − X₀`, with ξ and η the series in the conformal latitude and the longitude from the origin's meridian, and `X₀` the series' northing at the origin's latitude.
- Convergence `γ = γ₀ + atan2(q, p)`, with p and q the series' derivatives; near enough `γ ≈ (λ − λ₀)·sin φ`, which the invariant test uses as the sanity check.
- Point scale factor `k = k₀·B₁·k′·|p + iq|`.
- Parameters: the system's `φ₀` and `λ₀`, `k₀ = 0.9999`, no false easting or northing, GRS 80.

## Symbols and units

Latitude and longitude in degrees on JGD2011, north and east positive; X (northing) and Y (easting) in meters from the system's origin, negative to the south and west; convergence in degrees; the point scale factor dimensionless; the system a whole number 1 to 19.

## Domain

- Systems 1 to 19. Any other is refused by the field's declared range, which names it.
- The system is required: the tool does not guess it from the longitude.
- Outside JGD2011's published area, about 122°E to 158°E and 17°N to 46°N, the arithmetic still holds and the answer carries `OUTSIDE_AREA_OF_USE`.
- Heights are untouched.

## Approximations

The coordinates are taken to be on JGD2011, and nothing checks that they are. That is the `REALIZATION_ASSUMED` warning. JGD2000 uses the same systems with the same parameters, so the same latitude and longitude gives the same X and Y on both; what differs is the datum, which the 2011 Tohoku earthquake moved by up to about 5 m in eastern Japan. A Tokyo-datum latitude and longitude is on the Bessel ellipsoid, about 400 m away, and is not this grid. GSI's grids for moving it to JGD2000 or JGD2011 need Survey Act approval to use, so they are not in this tool.

A point projected on the wrong system gives valid numbers on that system, with a point scale factor further from 0.9999. The tool cannot tell, because only the prefecture decides.

## Worked example

- sourcePublisher: Ministry of Land, Infrastructure, Transport and Tourism of Japan and IOGP, through PROJ
- sourceTitle: JGD2011 / Japan Plane Rectangular CS IX, registered as EPSG 6677
- sourceEdition: MLIT Notice No. 9 of 2002, last amended 2010, read 2026-11-16; EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: System IX's own parameters, applied by PROJ to Tokyo Station at 35.681236°N 139.767125°E on JGD2011
- independent: yes
- inputs: lat 35.681236, lon 139.767125, zone 9
- outputs: X -35363.238 m, Y -5992.920 m
- tolerance: 0.001 m against PROJ, and 1e-6 m against GeographicLib
- verifiedBy: golden vectors v011 and v023 and `core/crates/gp-geodesy/tests/projections.rs` `jgd2011_round_trips_every_system_and_pins_the_notices_origins`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_jgd.py` builds 23 of the 32 vectors in `core/vectors/geodesy.projection.jgd2011-forward.jsonl` from PROJ 9.9.0 through `cs2cs`, applying each system's own EPSG record (checked against `projinfo` and the notice's origins on every run): one or more places in every one of the nineteen systems, from Ishigaki to Nemuro and Okinotorishima to Minamitorishima, within a millimetre. One more row projects London on system IX and requires `OUTSIDE_AREA_OF_USE`.
- `tools/vectors/gen_jgd.py` builds 5 from GeographicLib's TransverseMercatorProj, a second reference: the convergence, the point scale factor, and the coordinates to a micrometre, with the origin's own northing subtracted. The tight tolerance catches a WGS 84 fallback, which moves a coordinate by less than the millimetre the PROJ rows allow.
- The last 4 pin the notice's origins for systems I, IX, X, and XVIII: X and Y zero, the convergence zero, the scale factor 0.9999, and the EPSG code named.

## Invariants

- `core/crates/gp-geodesy/tests/projections.rs` `jgd2011_round_trips_every_system_and_pins_the_notices_origins`: all nineteen origins give X and Y zero, zero convergence, and 0.9999; a point north-east of each origin has positive X and Y, so the axes are not swapped; it round-trips through the inverse to a nanodegree with the same convergence and scale factor; the convergence is about `(λ − λ₀)·sin φ` in degrees; and Tokyo Station matches PROJ to a tenth of a millimetre.
- `core/crates/gp-geodesy/tests/projections.rs` `jgd2011_refuses_a_system_outside_the_nineteen_and_flags_a_point_off_japan`: systems 0, 20, and -9 are refused on `/zone`; a forward request without a system is refused rather than guessed; and a point off Japan answers with `OUTSIDE_AREA_OF_USE`.
