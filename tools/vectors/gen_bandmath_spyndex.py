#!/usr/bin/env python3
"""Band math against spyndex (promotion of raster.index.band-math).

The Awesome Spectral Indices catalog (Montero and others, Scientific Data 10,
197, 2023) publishes each index's formula as an expression, written
independently of geoprims. Its formulas are fed to raster.index.band-math as
they are, after one mechanical translation, because band math has no power
operator: x**0.5 becomes sqrt(x) and x**2 becomes (x)*(x). A formula with any
other power or any function call is skipped rather than guessed at. The
expected value is spyndex.computeIndex on the same bands, with each catalog
constant at its published default.

Writes core/crates/gp-raster/tests/data/bandmath_spyndex.json. Requires
spyndex."""
import ast
import json
import math
import random
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
import spyndex  # noqa: E402

FIX = Path("core/crates/gp-raster/tests/data/bandmath_spyndex.json")
CASES_PER_INDEX = 10


class Skip(Exception):
    pass


def emit(node):
    """Band-math text for a Python expression tree, or Skip."""
    if isinstance(node, ast.Expression):
        return emit(node.body)
    if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)):
        return repr(float(node.value))
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
        return f"(-{emit(node.operand)})"
    if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.UAdd):
        return emit(node.operand)
    if isinstance(node, ast.BinOp):
        ops = {ast.Add: "+", ast.Sub: "-", ast.Mult: "*", ast.Div: "/"}
        if type(node.op) in ops:
            return f"({emit(node.left)} {ops[type(node.op)]} {emit(node.right)})"
        if isinstance(node.op, ast.Pow) and isinstance(node.right, ast.Constant):
            base = emit(node.left)
            if node.right.value == 0.5:
                return f"sqrt({base})"
            if node.right.value == 2:
                return f"({base} * {base})"
    raise Skip(ast.dump(node)[:60])


def main():
    rng = random.Random(1112)
    constants = spyndex.constants
    out = []
    skipped = []
    for name in sorted(spyndex.indices):
        idx = spyndex.indices[name]
        try:
            expression = emit(ast.parse(idx.formula, mode="eval"))
        except (Skip, SyntaxError):
            skipped.append(name)
            continue
        cases = []
        for _ in range(CASES_PER_INDEX * 5):
            if len(cases) == CASES_PER_INDEX:
                break
            params = {}
            for b in idx.bands:
                if b in constants:
                    d = constants[b].default
                    if not isinstance(d, (int, float)):
                        break
                    params[b] = float(d)
                else:
                    params[b] = round(rng.uniform(0.01, 0.6), 4)
            else:
                try:
                    v = float(spyndex.computeIndex(name, params=params))
                except Exception:
                    continue
                if math.isfinite(v) and abs(v) < 1e6:
                    cases.append({"bands": [{"name": k, "value": x} for k, x in params.items()], "value": v})
        if len(cases) == CASES_PER_INDEX:
            out.append({"index": name, "formula": idx.formula, "expression": expression, "cases": cases})
        else:
            skipped.append(name)
    FIX.write_text(json.dumps({
        "source": f"spyndex {spyndex.__version__} (Awesome Spectral Indices)",
        "skipped": skipped,
        "indices": out,
    }, separators=(",", ":")) + "\n")
    print(f"{FIX}: {len(out)} indices, {len(out) * CASES_PER_INDEX} cases; {len(skipped)} skipped")
    append_vectors(out)


VECTORS = Path("core/vectors/raster.index.band-math.jsonl")
PICKS = ["NDVI", "EVI", "SAVI", "MSAVI", "BAI", "OSAVI", "VARI", "ARVI", "GNDVI", "CIG"]


def append_vectors(out):
    """Ten catalog formulas as golden vectors, after the existing lines, once."""
    lines = VECTORS.read_text().splitlines()
    if any("spyndex" in line for line in lines):
        return
    by_name = {i["index"]: i for i in out}
    for name in PICKS:
        idx = by_name[name]
        case = idx["cases"][0]
        n = len(lines) + 1
        lines.append(json.dumps({
            "id": f"v{n:03d}",
            "input": {"expression": idx["expression"], "bands": case["bands"]},
            "expect": {"result.value": case["value"], "ok": True},
            "source": f"Awesome Spectral Indices {name} formula ({idx['formula']}), evaluated by spyndex.computeIndex",
            "sourceVersion": f"spyndex {spyndex.__version__}",
            "tolerance": {"result.value": {"rel": 1e-12, "abs": 1e-12}},
        }, separators=(",", ":")))
    VECTORS.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
