<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Band math (`raster.index.band-math`)

## Method

The expression is lexed and parsed by precedence climbing into a tree, then the tree is evaluated in the core over the named bands. The language has numbers, the bands given, the arithmetic and comparison operators, a conditional (`test ? a : b`), and the functions abs, sqrt, log, exp, min, max, and clamp. Nothing else is recognized, and an unknown name is refused by name. The expression never reaches a JavaScript evaluator.

## Equations

- Precedence, low to high: `?:`, comparisons, `+ −`, `* /`, unary minus, then function calls and parentheses
- Arithmetic is IEEE 754 binary64, evaluated in the order written
- A result that is not finite (a division by zero, or the logarithm of a negative) is refused rather than returned

## Symbols and units

Bands are named numbers, usually surface reflectance on a 0 to 1 scale, and the result is a number in whatever units the expression implies.

## Domain

Any expression the language can parse, up to about a thousand nodes, over the bands passed in. It evaluates one pixel's values, not an image.

## Approximations

None beyond binary64 rounding: the tool computes exactly what the expression says, in the order it says it.

## Worked example

- sourcePublisher: Montero, D., and others, Awesome Spectral Indices (Scientific Data 10, 197, 2023)
- sourceTitle: spyndex, the Python front end of the Awesome Spectral Indices catalog, whose index formulas are published as expressions
- sourceEdition: spyndex 0.12.0
- sourceLocator: each catalog index's `formula`, evaluated by `spyndex.computeIndex` with the catalog's default constants
- independent: yes
- inputs: 267 of the catalog's 280 formulas, as written, with `x**0.5` spelled `sqrt(x)` and `x**2` spelled `(x)*(x)` (the 13 needing other powers or functions are skipped), at 10 random band sets each (0.01 to 0.6)
- outputs: each formula's value
- tolerance: 1e-12 relative
- verifiedBy: `core/crates/gp-raster/tests/indices.rs` `band_math_matches_spyndex_formulas` (2,670 cases); golden vectors v013 to v022
- verifiedOn: 2026-09-24

The formulas were written by the catalog's authors, not for this tool, so they exercise the parser on expressions nobody here chose: nested parentheses, unary minus, and constants such as EVI's g, C1, C2, and L.

## Differential tests

- `tools/vectors/gen_bandmath_spyndex.py`: the catalog formulas above, translated mechanically through Python's own parser, and their spyndex values
- `tools/vectors/gen_raster.py`: the paper formulas for NDVI, EVI, and others written as expressions, with precedence, conditionals, and clamp (golden vectors v001 to v012)

## Invariants

- `core/crates/gp-raster/tests/indices.rs` `band_math_invariants`: renaming the bands, reordering them, adding redundant parentheses, or swapping the operands of a sum or product leaves the value bit for bit the same. `band_math_evaluates_only_what_the_language_has` and `band_math_refuses_pathological_expressions` check the language's limits
