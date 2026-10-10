<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Legacy survey units (`survey.land.legacy-units`)

## Method

Old deeds, plats, and field notes give lengths in chains and links, rods, furlongs, varas, and arpents. The tool reads a length as written, which may mix units ("12 chains 34 links"), multiplies each count by its unit's length in US survey feet, and adds the parts. It reports the total in US survey feet, international feet, and meters, with the definition of each unit it used. The chain family has one definition everywhere. A vara or an arpent depends on where the record was made, so those need a jurisdiction and are refused without one.

## Equations

- 1 chain = 66 ft; 1 link = 0.66 ft (1/100 chain); 1 rod, pole, or perch = 16.5 ft (1/4 chain); 1 furlong = 660 ft (10 chains). The feet are US survey feet.
- Vara: Texas 33⅓ in; California and Florida 33.372 in.
- Arpent (linear): Louisiana 191.994 ft; Missouri 192.5 ft.
- Total in US survey feet = Σ count × unit length.
- Meters = US survey feet × 1200/3937. International feet = meters ÷ 0.3048.

## Symbols and units

Counts are plain numbers and may carry thousands separators. The US survey foot is 1200/3937 m exactly, and the international foot 0.3048 m exactly; they differ by 2 parts per million.

## Domain

A length made of one or more count-and-unit pairs from the units above. A vara or arpent without a jurisdiction is INVALID_INPUT at the jurisdiction, with the choices named. The result carries LEGACY_UNIT so a reader checks the record's own statement of units.

## Approximations

The chain family is exact by definition. NIST deprecated the US survey foot at the end of 2022 but keeps it for historic and legacy applications, which is what this tool is for; these units were defined on it before then. A vara or arpent is the commonly cited value for its jurisdiction: old surveyors sometimes used a worn chain or a local vara, older California records range down to 32.953 inches, and the record's own statement governs. Lengths only, not areas.

## Worked example

- sourcePublisher: National Institute of Standards and Technology
- sourceTitle: NIST Handbook 44, Specifications, Tolerances, and Other Technical Requirements for Weighing and Measuring Devices
- sourceEdition: 2026 edition, read in the PDF on 2026-10-09
- sourceLocator: Appendix C, section 2, "Gunter's or Surveyors Chain Units of Measurement": 1 link = 0.66 foot; 1 rod, perch, or pole = 25 links = 16.5 feet; 1 chain = 66 feet = 4 rods = 100 links; 1 furlong = 660 feet = 10 chains = 40 rods; 1 mile = 5,280 feet = 80 chains = 320 rods. Its note 4 says these units were defined on the U.S. survey foot before 2023
- independent: yes
- inputs: 1 link; 25 links; 1 chain; 10 chains; 80 chains; 320 rods
- outputs: 0.66, 16.5, 66, 660, 5,280, and 5,280 US survey feet
- tolerance: 1e-9 ft
- verifiedBy: golden vectors v008 through v020 and v006, run by the core on every build
- verifiedOn: 2026-10-09

The independent example covers the chain family. The vara and arpent values are taken from compiled surveying references named in the tool's citations, and were not checked against the statutes themselves for this note.

## Differential tests

- `tools/vectors/gen_survey.py`: the unit arithmetic worked again in Python for chains and links, rods, a furlong, a Texas and a California vara, and a Louisiana arpent
- `core/vectors/survey.land.legacy-units.jsonl`: those vectors, and 13 that hold the chain family to each line of the NIST table, in US survey feet and in meters, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/land.rs` `legacy_units_invariants`: a chain is 100 links, 4 rods, and a tenth of a furlong; a length is linear in its count; a mixed length is the sum of its parts; and the three outputs are one length in three units
- `core/crates/gp-survey/tests/land.rs` `vara_requires_jurisdiction`: a vara without a jurisdiction is refused at the jurisdiction with the choices named, and a Texas vara is 100/36 ft
