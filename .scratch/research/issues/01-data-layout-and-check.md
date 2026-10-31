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

## Comments

Follow-ups from review, none blocking:

- A dataset range's recipe lives beside the data files in the local evidence store, and its work row states the query. A later rule could pin a hash of the recipe, so a summary can be rebuilt without reading the prose query.
- The check holds honest authoring mistakes, not adversarial inputs. Review past that line kept finding new rules without finding wrong data, so a pass reports what would block a merge and lists the rest here.
