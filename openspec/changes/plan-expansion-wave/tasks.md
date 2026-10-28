## 1. Sequencing

- [x] 1.1 Add a pointer to this change under "Order of work" in `AGENTS.md`; verify the file names the nine changes in W1's order
- [ ] 1.2 At the 90-day review (December 22, 2026), re-rank changes 5 to 9 from the SEO log and problem reports; verify the decision record is committed with this change's W1 table updated

## 2. Budget checks

- [x] 2.1 Before starting change 6 or 7, measure `gp-navigation` with the tide engine and `gp-astro` linked; verify the measurement is recorded in the change's design and the split decision made (done so far, 2026-10-20: with the tide engine linked, `navigation.wasm` is 255,608 bytes Brotli of the 409,600 budget, recorded in add-marine-navigation's design; no split. Then 2026-10-28, with `gp-astro` linked by the five celestial tools: `navigation.wasm` is 939,589 bytes, 360,110 Brotli (88% of the budget), so no split, and the decision is recorded in add-marine-navigation's design. Two module readings worth carrying forward: `time.wasm` is 315,662 Brotli (77%) with the moon tools, and `survey.wasm` is 402,536 (98.3%), which leaves 7,064 bytes. Survey is the module to measure before writing into, not after)
- [ ] 2.2 Before starting change 9, confirm each asset's license text at the issuer and record it in the registry; verify `/licenses` renders every attribution in a preview build
