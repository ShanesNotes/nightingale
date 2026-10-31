#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 -I tools/gate/data.py check
python3 -I tools/gate/data.py probes
