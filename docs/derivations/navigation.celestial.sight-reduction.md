<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Sight reduction by the intercept method (`navigation.celestial.sight-reduction`)

## Method

A sextant sight fixes the observer on a circle of equal altitude centred on the body's geographic position. The intercept method (Marcq St Hilaire) replaces the short arc of that circle near the observer with its tangent: assume a position, compute the altitude and azimuth the body would have there, and shift the line by the difference between the observed and computed altitudes.

1. The body's apparent place comes from the same `gp-astro` series as the almanac tool, so the two can never disagree: VSOP87D for the Sun and planets, ELP 2000-82B for the Moon, Hipparcos for the 57 navigational stars and Polaris, carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation. GHA = GAST − α.
2. The local hour angle is the Greenwich hour angle plus the assumed longitude, east positive.
3. The computed altitude and the true azimuth follow from the navigational triangle (Bowditch Appendix B).
4. The intercept is the observed altitude less the computed altitude, read as nautical miles because a minute of arc of altitude is a mile of distance along the azimuth.
5. The line of position is drawn square to the azimuth through the point that far from the assumed position, toward the body when the observed altitude is the greater.

With `position_rule` set to `pub229`, the assumed position is rounded the way the sight reduction tables are entered: the nearest whole degree of latitude, and the longitude moved by less than half a degree so the local hour angle is a whole degree. Both values used are reported, so the work form can be checked against the tables.

## Equations

- LHA = (GHA + λ) mod 360°, λ east positive.
- Hc = asin(sin L sin d + cos L cos d cos LHA).
- Zn = atan2(−cos d sin LHA, cos L sin d − sin L cos d cos LHA), reduced to [0°, 360°).
- Z, the azimuth angle from the elevated pole: Zn or 360° − Zn in north latitude, |180° − Zn| in south, suffixed E when LHA > 180° and W when it is less. The quadrant rule the result states is the inverse: Zn = Z, 360° − Z, 180° − Z, or 180° + Z.
- a = (Ho − Hc) × 60, in nautical miles, Toward when positive.
- The intercept point is the assumed position carried a nautical miles along Zn (or its reciprocal when Away) on the WGS 84 geodesic; the plotted line runs 30 NM each way from it along Zn ± 90°.

## Symbols and units

L is the assumed latitude, λ the assumed longitude, d the declination, Ho the observed altitude, Hc the computed altitude, Zn the true azimuth, Z the azimuth angle, and a the intercept. Angles are output in degrees, the intercept in nautical miles. Trigonometry runs in radians.

## Domain

A UTC time from 1800-01-01 to 2100-12-31 (OUT_OF_DOMAIN otherwise); DUT1 within ±0.9 s; an observed altitude within ±90°; a body named among the Sun, the Moon, Venus, Mars, Jupiter, Saturn, and the 58 almanac stars (INVALID_INPUT otherwise). A negative computed altitude carries BODY_BELOW_HORIZON and is still reported, because an impossible sight is usually a clock or a name entered wrong, and the numbers show which.

## Approximations

- **The tangent for the circle:** the intercept is a straight-line stand-in for a circle of equal altitude whose radius is the zenith distance. The error grows with altitude and with distance from the azimuth line; above about 70° Bowditch advises working from the dead-reckoning position and keeping the line short, or plotting the circle instead. The tool does not plot circles.
- **Geocentric places:** declination and hour angle are geocentric, the almanac convention. The observer's parallax belongs to the altitude correction, not here.
- **Refraction:** none is applied. The observed altitude is already corrected.
- **One sight, one line:** no fix is computed. Crossing lines is the running-fix step.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Chapter 20, §2010, Table 2010b, the complete solution of a four-star fix on 2024-06-02
- independent: yes
- inputs: Spica at 2024-06-02T08:24:03Z, assumed position 41°00.0′S 128°18.7′E, Ho 32°53.4′
- outputs: LHA 304°00.0′, Dec S 11°17.4′, Hc 32°50.4′, Z S 104.6° E, Zn 075.4°, intercept 3.0 NM Toward. Regulus and Canopus are carried through the same way.
- tolerance: 0.2′ in LHA and Hc, 0.1° in Zn, 0.2 NM in the intercept. Every value the table prints is rounded to a tenth of a minute, and its Hc carries Pub. No. 229's own interpolation, so the printed digits cannot be reproduced closer than that.
- verifiedBy: `core/crates/gp-navigation/tests/celestial.rs` `bowditch_star_fix_reduced` and `assumed_position_rule_matches_the_table`, and golden vectors v001 to v003
- verifiedOn: 2026-10-27

Table 2010b's fourth column, Procyon, is left out of both the test and the vectors. Its increment of GHA Aries for 35m59s is printed as 8°56.2′ where 9°01.2′ is right, so its GHA, LHA, and computed altitude are each about 5′ adrift; USNO gives GHA 265°09.9′ for that second, not the table's 265°04.8′. The same column also repeats its GHA in the SHA row, as Table 2010a's correct 244°51.6′ shows.

## Differential tests

- `core/crates/gp-navigation/tests/celestial.rs` `sight_reduction_matches_usno` runs the tool against USNO's celnav API at 148 body-instants: 24 instants and places from 1995 to 2045, twelve bodies, with the API's own computed altitude and azimuth at the assumed position.
  - Sun, planets, and stars: worst 0.012′ in Hc and 0.001° in Zn.
  - The Moon: within 0.11′; Venus: within 0.19′. Both bodies are where USNO itself departs from JPL, as the almanac note records; the test allows 0.25′ and 0.5′.
  - Azimuth is not compared above 85° of altitude, where a tenth of a minute of altitude swings it degrees.
- `tools/vectors/gen_celestial_sights.py navigation.celestial.sight-reduction` builds the 29 vectors in `core/vectors/navigation.celestial.sight-reduction.jsonl`: three Bowditch columns, 24 USNO rows, and the below-horizon and unknown-body scenarios.

## Invariants

- `core/crates/gp-navigation/tests/celestial.rs` `at_the_geographic_position_the_intercept_vanishes` checks that standing at the Sun's own geographic position gives Hc 90° and, with Ho 90°, an intercept of zero.
- `core/crates/gp-navigation/tests/celestial.rs` `assumed_position_rule_matches_the_table` checks that the `pub229` rule returns a whole degree of latitude and a whole degree of local hour angle, and that the assumed longitude it picks is the one Table 2010b used.
- `core/crates/gp-navigation/tests/celestial.rs` `below_the_horizon_still_answers` checks that a negative computed altitude carries BODY_BELOW_HORIZON and still reports Hc and Zn.
- `core/crates/gp-navigation/tests/celestial.rs` `correcting_then_reducing_one_sight` checks that the correction tool's observed altitude, fed straight in, gives Bowditch's intercept within 0.2 NM.
