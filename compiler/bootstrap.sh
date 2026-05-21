#!/usr/bin/env bash
# bootstrap.sh — drive the stage-1 mom-in-mom compiler.
#
# Usage:   ./compiler/bootstrap.sh <source.mom> [-o <out_binary>]
#
# Pipeline:
#   1. stage-0 (cargo-built `mom`) runs `compiler/src/main.mom`.
#   2. The mom code reads $MOM_INPUT, lexes/parses/emits C, writes to $MOM_OUTPUT.
#   3. The host C compiler links the generated C against runtime/runtime.c.
#   4. The resulting native binary is written to the requested output.
#
# This script is the canonical demonstration that mom can compile itself
# (modulo the stage-1.0 subset documented in compiler/src/main.mom).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MOM_BIN="${MOM_BIN:-$ROOT/target/debug/mom}"
CC="${CC:-cc}"

if [ ! -x "$MOM_BIN" ]; then
    echo "stage-0 mom not built. Run: cargo build" >&2
    exit 1
fi

SOURCE=""
OUTPUT=""

while [ $# -gt 0 ]; do
    case "$1" in
        -o|--output)
            OUTPUT="$2"
            shift 2
            ;;
        -h|--help)
            sed -n '/^# Usage/,/^# This/p' "$0"
            exit 0
            ;;
        --)
            shift
            break
            ;;
        -*)
            echo "unknown flag: $1" >&2
            exit 1
            ;;
        *)
            if [ -z "$SOURCE" ]; then
                SOURCE="$1"
                shift
            else
                echo "unexpected argument: $1" >&2
                exit 1
            fi
            ;;
    esac
done

if [ -z "$SOURCE" ]; then
    echo "usage: $0 <source.mom> [-o <out_binary>]" >&2
    exit 1
fi

if [ -z "$OUTPUT" ]; then
    stem="$(basename "$SOURCE" .mom)"
    mkdir -p "$ROOT/target/stage1"
    OUTPUT="$ROOT/target/stage1/$stem"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

C_OUT="$WORK/stage1.c"

MOM_INPUT="$SOURCE" MOM_OUTPUT="$C_OUT" "$MOM_BIN" run "$ROOT/compiler/src/main.mom"

"$CC" -std=c99 -O0 -I "$ROOT/runtime" "$C_OUT" "$ROOT/runtime/runtime.c" -o "$OUTPUT"

echo "stage-1 build complete: $OUTPUT"
