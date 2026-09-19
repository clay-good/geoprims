#!/bin/sh
# Subsets Geist Sans and Geist Mono (SIL OFL 1.1) into apps/web/public/fonts
# (web/visual-theme "Self-hosted fonts and icons", budget 80 KB). Committed
# outputs; rerun only to change the character set or the Geist version.
#
# Source: npm geist 1.7.2, https://registry.npmjs.org/geist/-/geist-1.7.2.tgz
# SHA-256 of the tarball: 88cbfaca51646078f3172802643691bb8fe2df15ca4c455b1b101e49b7d469a6
# Needs: pyftsubset (pip install fonttools brotli).
set -eu
SRC=${1:?usage: subset-fonts.sh <unpacked geist package dir>}
OUT=$(dirname "$0")/../../apps/web/public/fonts
# Latin, Latin-1, dashes and quotes, primes, super- and subscripts, arrows,
# math symbols, Greek letters used in formulas, and the command key.
U="U+0020-007E,U+00A0-00FF,U+0131,U+0152-0153,U+0394,U+03A3,U+03A9,U+03B1-03C9,U+2010-2027,U+2030-203A,U+2044,U+2070-2079,U+2080-2089,U+20AC,U+2190-2199,U+2202,U+2206,U+2211-2212,U+2215,U+221A,U+221E,U+2248,U+2260,U+2264-2265,U+2318,U+2032-2033"
FEATURES='kern,liga,calt,tnum,case,frac,sups,subs'
pyftsubset "$SRC/dist/fonts/geist-sans/Geist-Variable.woff2" --unicodes="$U" --flavor=woff2 --layout-features="$FEATURES" --output-file="$OUT/Geist-Variable-subset.woff2"
pyftsubset "$SRC/dist/fonts/geist-mono/GeistMono-Variable.woff2" --unicodes="$U" --flavor=woff2 --layout-features="$FEATURES" --output-file="$OUT/GeistMono-Variable-subset.woff2"
cp "$SRC/LICENSE.txt" "$OUT/OFL.txt"
