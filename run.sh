#!/usr/bin/env bash
# Kagg2 launcher. Usage:
#   ./run.sh                 normal game (main menu)
#   ./run.sh fight [sector]  jump straight into a sector with enemies attacking (default: mars)
#   ./run.sh dock  [sector]  start docked at the first friendly starbase (trade/shipyard UI)
#   ./run.sh board [1-4] [size]  boarding level: 1 corvette 2 station 3 prison barge 4 dreadnought, size 1-5
#   ./run.sh tab N [sector]  start with station tab N open (1 helm .. 7 map)
#   ./run.sh sector NAME     fly a sector: mars phobos belt jupiter luna earth
#   ./run.sh viewer [model]  model viewer
#   ./run.sh test            fmt check, clippy, tests
#   ./run.sh shot OUT.png [extra game args]   save a screenshot and exit
set -euo pipefail
cd "$(dirname "$0")"
mode="${1:-game}"
[ $# -gt 0 ] && shift
run() { cargo run --release --quiet -- "$@"; }
case "$mode" in
  game)   run ;;
  fight)  run --space "${1:-mars}" --fight ;;
  dock)   run --space "${1:-mars}" --dock ;;
  board)  run --board "${1:-1}" --size "${2:-3}" ;;
  tab)    run --space "${2:-mars}" --tab "${1:-2}" ;;
  sector) run --space "${1:-mars}" ;;
  viewer) run --viewer "$@" ;;
  shot)   out="${1:-shot.png}"; [ $# -gt 0 ] && shift; run --shot "$out" "$@" ;;
  test)   cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --release ;;
  *) sed -n '2,12p' "$0"; exit 1 ;;
esac
