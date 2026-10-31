# 01: Public data and the check over it

Type: task
Status: open
Blocked by: none

**What to build:** `data/` holds the records behind the reviewed packet and healthy-range set as sorted JSON lines, and `tools/verify.d/30-data.sh` runs a stdlib check over them. Field sets are closed and text is capped, so a source passage has nowhere to live.

- [ ] Every rule has a must-fail probe, and probes refuse to run on a failing base
- [ ] Units parse through an exact table; a derivation is recomputed with dimensions carried
- [ ] An anchor refuses an inference anywhere in its derivation (I16); a compiled or textbook record caps certainty and names its locator
- [ ] Each identity in `data/closure.jsonl` closes within its stated tolerance
- [ ] A `Data:` line in a ticket that names no record fails
- [ ] `docs/governance.md` section 2, the `AGENTS.md` table and the verify skill name the new path and step
