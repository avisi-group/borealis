#!/usr/bin/env bash
set -e

### Helper script for rendering the control flow graphs to SVG.

# export genc from bincode
cargo r -- --log info,borealis::codegen::functions=trace sail2genc data/arm-v8.5-a.bincode.lz4 target/genc/ || true

# render
for filename in ./target/dot/*.dot; do
dot -Tsvg "$filename" > $filename.svg
done
