#!/usr/bin/env bash
# The workspace against the contract's crate table and the rules that keep each crate's build and tests
# inside its own directory; then every rule's must-fail probe and the plan and scan cases.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 -I tools/gate/gate.py seam
