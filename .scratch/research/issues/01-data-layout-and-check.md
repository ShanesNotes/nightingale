# 01: Public data and the check over it

Type: task
Status: resolved
Claimed by: claude
Blocked by: none

**What to build:** `data/` holds the records behind the reviewed packet and healthy-range set as sorted JSON lines, and `tools/verify.d/30-data.sh` runs a stdlib check over them. Field sets are closed and text is capped, so a source passage has nowhere to live.

- [x] Every rule has a must-fail probe, and probes refuse to run on a failing base
- [x] Units parse through an exact table; a derivation is recomputed with dimensions carried
- [x] An anchor refuses an inference anywhere in its derivation (I16); a compiled or textbook record caps certainty and names its locator
- [x] Each identity in `data/closure.jsonl` closes within its stated tolerance
- [x] A `Data:` line in a ticket that names no record fails
- [x] `docs/governance.md` section 2, the `AGENTS.md` table and the verify skill name the new path and step

## Answer

`data/` holds five files of sorted, canonical JSON lines: `works`, `packet-reference-man`, `healthy-range-set`, `inferences` and `closure`. `tools/gate/data.py` holds the rules and their probes, stdlib only, and `tools/verify.d/30-data.sh` runs both on `data/**` and on ticket Markdown, which stays docs-only.

- **Records.** A row names its id, kind, unit, value, population, conditions, method, window and evidence. Evidence takes one form: a located source with tier, reading and certainty; a derivation or an interval by a named identity; an inference record; or the anchors a calibrated value is fitted to.
- **Rules.** There are 18, and `data.schema` gates the rest. A rule that cannot read a record reports a violation, so a malformed row never passes and never crashes the gate.
- **Probes.** Each of the 195 breaks a synthetic base or a real file and names the rule that must refuse it. They refuse to run on a failing base. A mutation sweep outside the repo weakened the rules 123 ways and the probes caught every one.
- **Seed.** 19 records, five works and three closures, enough to exercise each rule on real values. No crate reads `data/`; [Reviewed packet builder](10-reviewed-packet-builder.md) scopes how a packet reaches one.

## Comments

Follow-ups from review, none blocking:

- A dataset range's recipe lives beside the data files in the local evidence store, and its work row states the query. A later rule could pin a hash of the recipe, so a summary can be rebuilt without reading the prose query.
- The check holds honest authoring mistakes, not adversarial inputs. Review past that line kept finding new rules without finding wrong data, so a pass reports what would block a merge and lists the rest here.
- Scaled cell counts (`10^9/L`) and assay activity units (`uU/mL`) are not in `UNITS`, so a record using one fails closed until its unit enters with an exact factor.
