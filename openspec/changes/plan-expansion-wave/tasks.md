## 1. Sequencing

- [x] 1.1 Add a pointer to this change under "Order of work" in `AGENTS.md`; verify the file names the nine changes in W1's order
- [ ] 1.2 At the 90-day review (December 22, 2026), re-rank changes 5 to 9 from the SEO log and problem reports; verify the decision record is committed with this change's W1 table updated

## 2. Budget checks

- [x] 2.1 Before starting change 6 or 7, measure `gp-navigation` with the tide engine and `gp-astro` linked; verify the measurement is recorded in the change's design and the split decision made (done so far, 2026-10-20: with the tide engine linked, `navigation.wasm` is 255,608 bytes Brotli of the 409,600 budget, recorded in add-marine-navigation's design; no split. Then 2026-10-28, with `gp-astro` linked by the five celestial tools: `navigation.wasm` is 939,589 bytes, 360,110 Brotli (88% of the budget), so no split, and the decision is recorded in add-marine-navigation's design. Two module readings worth carrying forward: `time.wasm` is 315,662 Brotli (77%) with the moon tools, and `survey.wasm` is 402,536 (98.3%), which leaves 7,064 bytes. Survey is the module to measure before writing into, not after)
- [ ] 2.2 Before starting change 9, confirm each asset's license text at the issuer and record it in the registry; verify `/licenses` renders every attribution in a preview build

## 3. Where the wave stands

Written 2026-11-07, after building changes 7 and 8 and opening 9. Each line
says what is built, and what the remaining verification waits on, so the next
session starts from the frontier rather than rediscovering it.

- [x] 3.1 `add-equipment-profiles` (1): complete.
- [x] 3.2 `add-aviation-practical-additions` (2): complete, and nine of its tools are stable.
- [x] 3.3 `add-photogrammetry-classics` (3): complete, and three of its tools are stable.
- [x] 3.4 `add-survey-partition-and-adjustment` (4): complete.
- [ ] 3.5 `add-sampling-and-grids` (5): all seven tools built and checked against a Python reimplementation on PROJ and GEOS. What is left is parity with QGIS itself (tasks 3 and 5) and with R's `dssd` (task 6); neither is installed here and neither is in the differential container. The seven tools stay experimental until one of those runs, because the stable bar asks for an independent worked example and a reimplementation we wrote is not one.
- [ ] 3.6 `add-marine-navigation` (6): tasks 3, 4 and 6 are done; the tide engine matches NOAA within 0.009 ft. The sailings, piloting, tide and clearance tools are built, and four of them are stable. What is left: the `BELOW_MARGIN` and toolset-split halves of task 7 are done, and under-keel clearance stays experimental because the USACE notes that restate Barrass with worked examples (CHETN-I-72, CHETN-IX-19/20) are behind the ERDC and DTIC bot checks, and the one open restatement found prints a worked case its own equations do not reproduce.
- [x] 3.7 `add-celestial-and-moon` (7): complete, and nine of its tools are stable.
- [ ] 3.8 `add-raster-hydrology` (8): all six tools built, with tasks 7, 8 and 9 closed and the rest carrying their notes. Every tool is checked against a second implementation written in Python from the papers, each using a different method from the core's. What is left is task 2: WhiteboxTools and richdem in the differential container. Docker is not available on this machine and `pip install whitebox` is refused by its sandbox, so no oracle figure is claimed and the six tools stay experimental.
- [ ] 3.9 `add-international-grids` (9): opened. Built so far: the OS National Grid references both ways, Irish Transverse Mercator both ways, and the Swiss LV95 and LV03 grids by swisstopo's rigorous formulas, each against PROJ and against its issuer's own worked example where one is published. What is left needs the asset pipelines of tasks 1 and 2: OSTN15 and OSGM15, the OSi polynomial for the Irish Grid, GDA2020, NZGD2000, and the rest of task 6. Those are downloads with licenses to record, not code.
