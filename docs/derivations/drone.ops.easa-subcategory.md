<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# EASA open subcategory (`drone.ops.easa-subcategory`)

## Method

The EU open category has three subcategories, A1, A2, and A3, and which of them a drone may use depends on its class identification label, or, for a drone without one, on its takeoff mass. The tool is a lookup: it takes the class mark and the mass and lists the subcategories open to that drone, with the distance rule of each. It is not a calculation, and it reads its two numeric limits from dated entries in `data/regulations.json`.

## Equations

- C0 or C1: A1 and A3.
- C2: A2 and A3.
- C3 or C4: A3.
- No class mark, under 250 g: A1 and A3. No class mark, 250 g or more: A3.
- 25 kg or more: outside the open category (OUT_OF_DOMAIN).
- C5 or C6: the specific category under a standard scenario, not the open category (OUT_OF_DOMAIN).

## Symbols and units

Mass is the maximum takeoff mass with payload, in any mass unit (kg by default). The class mark is the label on the drone: none, or C0 to C6.

## Domain

Mass above zero and under 25 kg; class mark none or C0 to C4.

## Approximations

Two things are simplified, and the tool says both. It trusts the class mark and does not check it against the mass, so a C0 label on a heavy drone is taken at its word. And the regulation's own list for A3 (point UAS.OPEN.040(4)) names privately built and legacy drones under 25 kg and classes C2, C3, and C4; that a C0 or C1 drone may also fly under the stricter A3 conditions is EASA's published reading, shown in its class label table, not a line of the regulation. Height limits, pilot competency, geographical zones, and national rules are not checked. A drone without a label is treated the same whether privately built or placed on the market before 2024.

## Worked example

- sourcePublisher: European Union Aviation Safety Agency
- sourceTitle: Open category: low risk civil drones (web page)
- sourceEdition: read 2026-10-10
- sourceLocator: the class label table: C0 "A1 (can also fly in subcategory A3)"; C1 "A1 (can also fly in subcategory A3)"; C2 "A2 (can also fly in subcategory A3)"; C3 and C4, A3; privately built or placed on the market before 01/01/2024, under 250 g "A1 (can also fly in subcategory A3)" and under 25 kg, A3
- independent: yes
- inputs: a 200 g C0, an 899 g C1, a 1.2 kg C2, a 5 kg C3, an 8 kg C4, and drones of 249 g and 250 g with no class mark
- outputs: A1 and A3; A1 and A3; A2 and A3; A3; A3; A1 and A3; A3
- tolerance: exact
- verifiedBy: golden vectors v007 through v017, run by the core on every build
- verifiedOn: 2026-10-10

The distance rules shown with each subcategory were checked against the consolidated text of Regulation (EU) 2019/947 on EUR-Lex the same day: A1 may overfly uninvolved people but never assemblies, and C1 should not expect to overfly anyone (UAS.OPEN.020); A2 keeps 30 m, or 5 m in low-speed mode (UAS.OPEN.030); A3 keeps 150 m from residential, commercial, industrial, or recreational areas (UAS.OPEN.040); drones without a label placed on the market before 1 January 2024 fly in A1 under 250 g and in A3 under 25 kg (Article 20).

## Differential tests

- `tools/vectors/gen_drone.py`: the expected subcategories written out by hand for every row of EASA's table, the 250 g and 25 kg lines, and the C5 and C6 refusals
- `core/vectors/drone.ops.easa-subcategory.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `easa_subcategory_invariants`: a labeled drone's answer depends on its class alone, an unlabeled one's turns at 250 g, A3 is always among them, one rule is listed per subcategory, and 25 kg, C5, and C6 are refused
- `core/crates/gp-drone/tests/power_ops.rs` `legacy_2_kg_drone`: the spec scenario, a 2 kg drone with no class mark, is A3 only
