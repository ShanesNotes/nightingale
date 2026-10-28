#!/usr/bin/env bash
# The solver-contract sketch: the pinned toolchain, a warning-free build, every doctest, the error
# code of each compile-fail doctest, and the coupling census against docs/atlas/coupling-map.md.
set -euo pipefail
cd "$(dirname "$0")/../.."
want=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
have=$(rustc --version | cut -d' ' -f2)
[ "$have" = "$want" ] || { echo "20-sketch: rustc $have, rust-toolchain.toml pins $want"; exit 1; }
sketch=docs/architecture/solver-contract-sketch
cd "$sketch"
RUSTFLAGS="-D warnings" cargo check --all-targets --locked
cargo test --doc --locked
# Stable rustdoc does not check a compile_fail block's error code, so a copy with the markers
# removed must fail each block with the code its marker names.
# lib.rs includes ../../solver-contract.md as its crate doc, so the copy keeps that layout.
copy=$(mktemp -d)
trap 'rm -rf "$copy"' EXIT
mkdir -p "$copy/architecture/sketch"
cp ../solver-contract.md "$copy/architecture/"
cp -r Cargo.toml Cargo.lock src "$copy/architecture/sketch/"
sed -i 's/```compile_fail,E[0-9]*/```/' "$copy/architecture/sketch/src/lib.rs"
(cd "$copy/architecture/sketch" && cargo test --doc --locked >"$copy/unmarked.log" 2>&1) || true
python3 -I - src/lib.rs "$copy/unmarked.log" <<'PY'
import re, sys
want = {i + 1: m.group(1) for i, line in enumerate(open(sys.argv[1], encoding='utf-8'))
        if (m := re.search(r'```compile_fail,(E\d+)', line))}
parts = re.split(r'---- src/lib\.rs - \(line (\d+)\) stdout ----', open(sys.argv[2], encoding='utf-8').read())
got = {int(parts[i]): set(re.findall(r'error\[(E\d+)\]', parts[i + 1])) for i in range(1, len(parts), 2)}
bad = sorted((line, code) for line, code in want.items() if code not in got.get(line, set()))
print(f'20-sketch: {len(want)} compile_fail doctests, {len(want) - len(bad)} fail with their declared code')
for line, code in bad:
    print(f'20-sketch: src/lib.rs:{line} does not fail with {code}')
sys.exit(1 if bad or not want else 0)
PY
python3 -I evidence/census-check.py
