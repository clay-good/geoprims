<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Sextant altitude corrections (`navigation.celestial.altitude-correction`)

## Method

What a sextant reads is not the altitude a sight reduction wants. Five corrections stand between them, and this tool applies them in the order Bowditch §1612 and §1613 set out, reporting each one:

1. **Index correction.** The reader's own, positive off the arc and negative on it.
2. **Dip of the horizon.** The sea horizon is below the true horizontal by the height of eye. Subtracted. Index correction and dip together take the sextant altitude to the apparent altitude, which every later correction is entered at.
3. **Refraction.** Bennett's (1982) formula, scaled for the pressure and air temperature measured at the sextant. Subtracted.
4. **Semidiameter.** A marine sextant brings a limb of the Sun or Moon to the horizon, not its centre. Added for a lower limb, subtracted for an upper, zero for a centre, a star, or a planet. The Sun's comes from its computed distance, the Moon's from its horizontal parallax; either can be overridden with the value from a printed almanac.
5. **Parallax in altitude.** The sight is taken from the Earth's surface and the almanac's places are geocentric. Added; minutes of arc for the Moon, a tenth of a minute for the Sun and planets, nothing for a star.

Below 10° of apparent altitude the result carries LOW_ALTITUDE_REFRACTION, because refraction there grows fast and varies with the air itself.

## Equations

- ha = hs + IC − D, with D = 1.76′ √h in metres (0.97′ √h in feet, the same constant).
- R₀ = cot(ha + 7.31 / (ha + 4.4)) arcminutes, ha in degrees (Bennett 1982).
- R = R₀ × (P / 1010 hPa) × (283 K / (273.15 + T °C)).
- SD: the Sun, 959.63″ / R with R in au; the Moon, asin(0.2725076 sin HP).
- HP = asin(6,378.137 km / Δ), with Δ the geocentric distance.
- PA = HP cos ha.
- Ho = ha − R ± SD + PA, the semidiameter added for a lower limb and taken off for an upper.

## Symbols and units

hs is the sextant altitude, IC the index correction, D the dip, ha the apparent altitude, R the refraction, SD the semidiameter, HP the horizontal parallax, PA the parallax in altitude, and Ho the observed altitude. Altitudes are output in degrees; every correction is in arcminutes, signed the way it is applied.

## Domain

A sextant altitude from −5° to 90°; a height of eye from 0 to 10,000 m; an air temperature above absolute zero and a positive pressure; a UTC time from 1800-01-01 to 2100-12-31. A limb other than the centre is refused for a star or a planet, which show no disc at a sextant's magnification. Defaults: 10 °C and 1010 hPa, the standard refraction conditions, and a zero index correction.

## Approximations

- **Bennett's formula** stands in for an integration through the atmosphere. It matches USNO's own refraction within 0.02′ from 1.4° to 89° of apparent altitude, so the formula is not what limits a low sight; the air is.
- **The pressure and temperature factor** is the usual linear one. It assumes the air near the sextant stands for the whole path, which is exactly what fails at low altitude.
- **Dip** is the geometric horizon with the standard terrestrial refraction coefficient built into the 1.76′ constant. Anomalous dip from a temperature inversion is not modelled and can reach minutes.
- **The Moon's semidiameter** is the geocentric one, the value the Nautical Almanac tabulates; it is not augmented for the observer's altitude. Its parallax uses the tabulated horizontal parallax and the equatorial radius, again the almanac convention. A rigorous topocentric reduction, as USNO computes, augments the semidiameter by up to 0.15′ and takes the parallax from the observer's own geocentric radius, up to 0.2′ smaller. Bowditch's four Moon examples agree with the almanac convention within 0.1′, so that is the one followed here, and USNO's Moon is left out of the vectors.
- **Not covered:** artificial horizons, artificial-horizon sextants, back sights, bubble sextants, and the Moon's upper-limb "additional" 30′ of the printed tables, which exists only because those tables run from a lower-limb base.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Chapter 16, Sextant Altitude Corrections, §1618 Examples 1 and 2, §1619 Examples 1 and 2, §1620, §1621, and §1622 Examples 1 and 3
- independent: yes
- inputs: the Sun's lower limb on 2024-06-02, hs 51°28.4′, index correction −2.0′, height of eye 38 ft
- outputs: dip −6.0′, ha 51°20.4′, total correction +15.2′, Ho 51°35.6′. The other seven examples are carried through the same way: the Sun's upper limb (Ho 32°25.1′), the Moon's lower and upper limbs with HP 59.6′ (19°12.2′ and 66°44.5′), Venus (44°15.3′), the star Miaplacidus (27°46.3′), and the two low sights of §1622 (1°15.4′ and 2°57.4′).
- tolerance: 0.05′ in the apparent altitude, 0.15′ in Ho above 5° of apparent altitude, and 0.25′ for the two sights below it. The Almanac's own total-correction tables are critical tables, so a printed value holds over a band of altitudes rather than at a point; the Sun's two examples sit 0.11′ from the value this tool computes, which is that band.
- verifiedBy: `core/crates/gp-navigation/tests/celestial.rs` `bowditch_chapter_16_examples`, and golden vectors v001 to v008
- verifiedOn: 2026-10-27

## Differential tests

- `tools/vectors/gen_celestial_sights.py navigation.celestial.altitude-correction` builds the 35 vectors in `core/vectors/navigation.celestial.altitude-correction.jsonl`: the eight Bowditch examples, 25 rows from USNO's celnav API, and the low-altitude and star-limb scenarios. USNO's refraction, semidiameter, and parallax are evaluated at the apparent altitude ha = hc − refr − sd − pa, so a sight from a zero height of eye with no index error has exactly that sextant altitude and the two can be compared term by term.
  - Refraction: within 0.02′ over the 1.4° to 89° the rows cover.
  - Semidiameter and parallax for the Sun and the planets: within 0.1′.
  - The observed altitude the chain returns: within 0.0025° (0.15′) of USNO's computed altitude.
  - The Moon is left out, for the convention difference the Approximations record.

## Invariants

- `core/crates/gp-navigation/tests/celestial.rs` `the_corrections_add_up` checks that the five reported corrections sum to the total, that the total takes the sextant altitude to the observed altitude, that the dip of a 38-foot height of eye is 6.0′, and that refraction and dip are reported negative.
- `core/crates/gp-navigation/tests/celestial.rs` `a_low_sight_says_so_and_a_star_has_no_limb` checks that a sight below 10° of apparent altitude carries LOW_ALTITUDE_REFRACTION, and that a star with a limb named is refused at `/limb`.
- `core/crates/gp-navigation/tests/celestial.rs` `correcting_then_reducing_one_sight` checks that Table 2010a's sextant altitude for Spica, with the example's sextant and height of eye, gives Table 2010b's observed altitude within 0.1′.
