#!/bin/sh
set -eu

fail() {
  echo "reference smoke failed: $1" >&2
  exit 1
}

geographiclib_version=$(GeodSolve --version)
case "$geographiclib_version" in
  *2.7*) ;;
  *) fail "GeographicLib version: $geographiclib_version" ;;
esac
geodesic=$(printf '0 0 0 1\n' | GeodSolve -i)
case "$geodesic" in
  *'90.00000000 90.00000000 111319.491'*) ;;
  *) fail "GeographicLib inverse result: $geodesic" ;;
esac

proj_version=$(proj 2>&1 || true)
case "$proj_version" in
  *9.9.0*) ;;
  *) fail "PROJ version: $proj_version" ;;
esac
projected=$(printf '0 0\n' | proj +proj=merc +ellps=WGS84)
case "$projected" in
  '0.00	0.00') ;;
  *) fail "PROJ Mercator result: $projected" ;;
esac

h3_version=$(h3 --help 2>&1)
case "$h3_version" in
  *4.5.0*) ;;
  *) fail "H3 version: $h3_version" ;;
esac
h3_cell=$(latLngToCell -r 9 --lat 37.775938728915946 --lng -122.41795063018799)
[ "$h3_cell" = "8928308280fffff" ] || fail "H3 cell: $h3_cell"

s2_cell=$(s2-smoke)
[ "$s2_cell" = "43ce61" ] || fail "S2 cell: $s2_cell"

wmm_output=$(mktemp)
trap 'rm -f "$wmm_output"' EXIT HUP INT TERM
(
  cd /usr/local/share/wmm2025
  wmm2025-file f sample_coords.txt "$wmm_output"
)
first_wmm_row=$(sed -n '2p' "$wmm_output")
case "$first_wmm_row" in
  *'16d 48m'*'52531.3'*) ;;
  *) fail "WMM2025 result: $first_wmm_row" ;;
esac

htdp_output=$(printf '0\n' | htdp)
case "$htdp_output" in
  *'SOFTWARE VERSION: 3.6.0'*) ;;
  *) fail "HTDP version banner" ;;
esac

printf '%s\n' \
  "GeographicLib 2.7: ok" \
  "PROJ 9.9.0: ok" \
  "H3 C 4.5.0: ok" \
  "S2 C++ 0.13.1: ok" \
  "NOAA WMM2025 C: ok" \
  "NGS HTDP 3.6.0: ok"
