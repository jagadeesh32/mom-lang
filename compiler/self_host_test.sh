#!/bin/bash
# self_host_test.sh — Tests self-hosting fixed-point for mom
#
# Pipeline:
#   1. stage-0 compiles stage-1 (main.mom) → stage1_native binary
#   2. stage1_native compiles stage-1 (main.mom) → stage1_native_v2 + v2.c
#   3. stage1_native_v2 compiles stage-1 (main.mom) → stage1_native_v3 + v3.c
#   4. Compare v2.c and v3.c — they should be identical (fixed point!)
#   5. Print PASS or FAIL

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STAGE0="${MOM_BIN:-$REPO_ROOT/target/debug/mom}"
COMPILER_SRC="$REPO_ROOT/compiler/src/main.mom"
RUNTIME_C="$REPO_ROOT/compiler/runtime.c"
RUNTIME_H_DIR="$REPO_ROOT/compiler"
CC="${CC:-gcc}"

echo "=== mom self-hosting fixed-point test ==="

# Preflight checks
if [ ! -x "$STAGE0" ]; then
    echo "FAIL: stage-0 binary missing: $STAGE0" >&2
    echo "      Run: cargo build" >&2
    exit 1
fi

if [ ! -f "$RUNTIME_C" ]; then
    echo "FAIL: runtime.c not found: $RUNTIME_C" >&2
    exit 1
fi

WORK_DIR=$(mktemp -d)
trap "rm -rf $WORK_DIR" EXIT

compile_mom_to_native() {
    local driver="$1"      # binary that drives compilation (stage-0 uses 'run', native uses direct)
    local driver_mode="$2" # "stage0" or "native"
    local input="$3"       # .mom source to compile
    local c_out="$4"       # output .c path
    local bin_out="$5"     # output binary path

    if [ "$driver_mode" = "stage0" ]; then
        MOM_INPUT="$input" MOM_OUTPUT="$c_out" "$driver" run "$COMPILER_SRC"
    else
        MOM_INPUT="$input" MOM_OUTPUT="$c_out" "$driver"
    fi

    "$CC" -std=c99 -O2 \
        -I"$RUNTIME_H_DIR" \
        "$c_out" "$RUNTIME_C" \
        -o "$bin_out"
}

# Step 1: stage-0 → stage1_native
echo ""
echo "[1/3] stage-0 compiles stage-1 → stage1_native"
C1="$WORK_DIR/stage1_v1.c"
BIN1="$WORK_DIR/stage1_native"
compile_mom_to_native "$STAGE0" "stage0" "$COMPILER_SRC" "$C1" "$BIN1"
echo "      Binary: $BIN1"

# Step 2: stage1_native compiles stage-1 → stage1_native_v2 + v2.c
echo ""
echo "[2/3] stage1_native compiles stage-1 → stage1_native_v2"
C2="$WORK_DIR/stage1_v2.c"
BIN2="$WORK_DIR/stage1_native_v2"
compile_mom_to_native "$BIN1" "native" "$COMPILER_SRC" "$C2" "$BIN2"
echo "      Binary: $BIN2"

# Step 3: stage1_native_v2 compiles stage-1 → stage1_native_v3 + v3.c
echo ""
echo "[3/3] stage1_native_v2 compiles stage-1 → stage1_native_v3"
C3="$WORK_DIR/stage1_v3.c"
BIN3="$WORK_DIR/stage1_native_v3"
compile_mom_to_native "$BIN2" "native" "$COMPILER_SRC" "$C3" "$BIN3"
echo "      Binary: $BIN3"

# Step 4: Compare v2.c and v3.c
echo ""
echo "[4/4] Comparing stage1_v2.c and stage1_v3.c..."
if diff -u "$C2" "$C3" > "$WORK_DIR/diff.txt" 2>&1; then
    echo ""
    echo "PASS: C output is identical at fixed point!"
    echo "      mom can compile itself reproducibly."
    exit 0
else
    echo ""
    echo "FAIL: C output differs between round 2 and round 3."
    echo "      This means the compiler is not yet at a fixed point."
    echo ""
    echo "--- diff (first 50 lines) ---"
    head -50 "$WORK_DIR/diff.txt"
    exit 1
fi
