<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# QNH, QFE, and QNE (`aviation.altimetry.q-codes`)

## Method

An altimeter set to QNH reads PA(p) − PA(QNH), and on the field it reads the elevation. So the field sits at the ISA altitude elevation + PA(QNH), and QFE, the station pressure, is the ISA pressure there. Going the other way, QNH is the ISA pressure at the station's pressure altitude less the elevation. QNE, the field's pressure altitude, is the ISA altitude whose pressure is QFE. When QFE is given, the tool also shows the altimeter setting by the NWS formula, the way US weather stations work it out.

## Equations

- From QNH: QFE = p_ISA(elevation + PA(QNH))
- From QFE: QNH = p_ISA(PA(QFE) − elevation)
- QNE = PA(QFE)
- PA(p) is the closed-form inverse of the ISA layer: T0/L × ((p/p0)^(−R L/g0) − 1) in the troposphere, and the matching isothermal form above 11 km
- NWS (from QFE only): QNH_NWS = (P − 0.3) · (1 + (1013.25^0.190284 × 0.0065 / 288) · h / (P − 0.3)^0.190284)^(1/0.190284), P in hPa, h in m

## Symbols and units

p0 = 101,325 Pa, T0 = 288.15 K, L = −0.0065 K/m, R = 287.05287 J/(kg·K), g0 = 9.80665 m/s². QNH accepts inHg, hPa, and METAR `A2992` or `Q1013` groups and comes back in inHg by default; QFE comes back in hPa; QNE and the elevation in feet.

## Domain

Field elevation from −1 km to 11 km (−3,281 ft to 36,089 ft). Give either QNH or QFE, not both (INVALID_INPUT). QNH from 10 to 40 inHg, with SUSPECT_VALUE outside 26 to 32 inHg; QFE from 100 to 1,100 hPa (OUT_OF_DOMAIN outside).

## Approximations

The relation follows the standard atmosphere and ignores today's temperature, which is how an altimeter setting is defined. The NWS formula is the same relation with its own constants and subtracts 0.3 hPa for the barometer's height, so it reads about 0.3 hPa (0.01 inHg) lower.

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite altimetry spec, scenario "QFE from QNH"
- sourceEdition: openspec/changes/add-aviation-suite/specs/aviation/altimetry/spec.md
- sourceLocator: Requirement "Q-code conversions"; the spec names the method, and the numbers are the ISA relation worked in Python by tools/vectors/gen_qcodes.py
- independent: no
- inputs: field elevation 1,000 ft, QNH 1013 hPa
- outputs: QFE 976.92 hPa, QNH 29.91 inHg, QNE 1,006.8 ft; with QFE set the altimeter reads PA(QFE) − PA(QFE) = 0 ft on the ground
- tolerance: 1e-6 relative
- verifiedBy: golden vector v010 (it supersedes v001, which came from a generator that shared the core's old pressure-ratio shortcut)
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_qcodes.py`: a separate Python ISA (troposphere and the 11 km isothermal layer) with its own inverse, the altimeter-setting relation, and the NWS formula from its published PDF, at four QNH cases and three QFE cases plus a both-given error (within 1e-6 relative)
- `core/vectors/aviation.altimetry.q-codes.jsonl`: those vectors (v010 to v015 supersede v001, v002, v004 to v007, which came from the old pressure-ratio shortcut; v003 at sea level and v008 still stand), the spec scenario, and a display check that QNH prints to hundredths of an inch (v016), run through the core on every build

## Invariants

- `core/crates/gp-aviation/src/qcodes.rs` `nws_formula_at_sea_level_returns_the_reading_less_its_offset`: at sea level the NWS formula returns the station pressure less its 0.3 hPa offset, within 1e-9 hPa
- `core/crates/gp-aviation/tests/aviation.rs` `altimetry_invariants`: exercises the same station-pressure relation through `aviation.altimetry.pressure-altitude`, not this tool: at 1013.25 hPa the pressure altitude equals the elevation, and it falls as the setting rises. No test runs a QNH to QFE to QNH round trip through this tool.
