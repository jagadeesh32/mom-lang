#!/bin/bash
# bootstrap.sh — Full self-hosting bootstrap for mom
# Usage: ./compiler/bootstrap.sh [input.mom] [output_binary]
#
# Pipeline:
#   1. stage-0 (cargo-built `mom`) runs `compiler/src/main.mom`.
#   2. The mom code reads $MOM_INPUT, lexes/parses/emits C, writes to $MOM_OUTPUT.
#   3. The host C compiler links the generated C against compiler/runtime.c.
#   4. The resulting native binary is written to the requested output.
#
# This script is the canonical demonstration that mom can compile itself
# (modulo the stage-1 subset documented in compiler/src/main.mom).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STAGE0="${MOM_BIN:-$REPO_ROOT/target/debug/mom}"
COMPILER_SRC="$REPO_ROOT/compiler/src/main.mom"
RUNTIME_C="$REPO_ROOT/compiler/runtime.c"
RUNTIME_H_DIR="$REPO_ROOT/compiler"
CC="${CC:-gcc}"

# Build stage-0 if needed
if [ ! -f "$STAGE0" ]; then
    echo "Building stage-0 interpreter..."
    cargo build --manifest-path "$REPO_ROOT/Cargo.toml" 2>&1
fi

if [ ! -x "$STAGE0" ]; then
    echo "stage-0 mom not built or not executable: $STAGE0" >&2
    echo "Run: cargo build" >&2
    exit 1
fi

if [ ! -f "$RUNTIME_C" ]; then
    echo "runtime.c not found: $RUNTIME_C" >&2
    echo "The runtime has not been written yet." >&2
    exit 1
fi

INPUT="${1:-$REPO_ROOT/compiler/examples/factorial.mom}"
OUTPUT="${2:-}"

if [ -z "$OUTPUT" ]; then
    stem="$(basename "$INPUT" .mom)"
    mkdir -p "$REPO_ROOT/target/stage1"
    OUTPUT="$REPO_ROOT/target/stage1/$stem"
fi

WORK_DIR=$(mktemp -d)
trap "rm -rf $WORK_DIR" EXIT

# Step 1: Stage-0 runs stage-1 compiler to produce C
C_OUTPUT="$WORK_DIR/output.c"
echo "==> Stage-0 interpreting stage-1 compiler..."
MOM_INPUT="$INPUT" MOM_OUTPUT="$C_OUTPUT" "$STAGE0" run "$COMPILER_SRC"
echo "    Generated: $C_OUTPUT"

# Step 2: Compile generated C + runtime -> native binary
echo "==> Compiling generated C + runtime..."
"$CC" -std=c99 -O2 \
    -I"$RUNTIME_H_DIR" \
    "$C_OUTPUT" "$RUNTIME_C" \
    -o "$OUTPUT"
echo "    Binary: $OUTPUT"

# Step 3: Run and show output
echo "==> Running..."
"$OUTPUT"
