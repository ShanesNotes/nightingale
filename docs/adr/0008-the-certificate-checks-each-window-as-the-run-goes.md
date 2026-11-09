# The certificate checks each window as the run goes, and a run keeps a verifiable chain, not every ledger chunk

Keeping every ledger chunk costs 296 to 342 GB for one 48 h course. Compressed, it still costs 74 to 86 GB, and few readers ever come back to it. So the certificate becomes a fold: it reads each step a session accepts, judges each sliding window when the step that holds its end is accepted, and gives the same certificate `certify` gives on the complete history, bit for bit. During the run, the history store keeps a rolling day of complete links. After the run it keeps a witness for each step, a checkpoint each simulated minute with its head link, and, when the run fails, the original links from the checkpoint before the failing window to the last head (the flight recorder). Anything else asked of a finished run later is answered by regeneration on the build that made it, checked step by step against the witnesses.

## Considered options

- Keep every chunk, compressed by delta and zstd (74 to 86 GB for 48 h): rejected. It keeps what this record gives up: anyone can recompute a certificate from the links without the core, `restore` reparses every ancestor, and a corrected policy is rechecked from the links. But it costs about twenty times what a passing run keeps here, and it does not fit the build machine.
- Stream with no flight recorder (3.2 to 3.6 GB after any run): rejected. A failure's original bytes would be gone, and those matter most when the core itself is suspect, since a faulty core may not regenerate.
- Bare 32-byte seals and checkpoints (1.1 to 1.2 GB): rejected. A seal cannot be recomputed from itself, so `restore` could not check the chain to admission, nor the checkpoint's image against it.
- Regenerate inside `certify`: rejected. `certify` would run a core and stop being pure, and every certificate would cost a rerun.

## Consequences

- A witness is 96 bytes: a step's seal, its image digest and its ledger digest. Each seal recomputes from the seal before it, the revision and those two digests, with the hash a run already uses. So a 48 h course keeps 3.05 to 3.52 GB of witnesses, plus 111 MB of checkpoints. During the run, the rolling day of compressed links takes 37 to 43 GB. These are by arithmetic from short windows; no 48 h course has been measured.
- Recomputing a certificate without the core is lost for a trimmed run. A third party must rerun the pinned build (about 7 wall hours for 48 h at measured speed) or receive a regenerated history. Only the flight recorder's span stays checkable locally.
- `restore` still checks every seal back to admission, the checkpoint's head link and its image. It no longer reparses the ledger of discarded steps, and a regeneration gives that check back.
- A corrected healthy-range set or policy is rechecked by regeneration. The one exception is a complete run that passed closure, under the same basis, windows and closure limits: its tally of the worst drift for each combination can show at once that it passes a loosened drift bound. A failure under the new bound still needs the rerun to name its first window.
- Every replay gate keeps its live comparison on fresh cores. A revision between checkpoints replays forward from the checkpoint before it.
- A new lens, a debrief and the readings checks needed a replay before this record, and still do, because no store holds past state or readings.
- A run's evidence names the build that made it. A regeneration that no longer reproduces its seals is itself a finding, and its run can still be compared against its witnesses step by step.
- The flight recorder starts on the first closure or drift failure, a replay divergence, an engine fault or a store failure. An early failure can keep nearly the whole course. While every core fails the synthetic energy drift, every milestone run keeps it.
- The fold reads one session through a view the record mints at admission, so no host can feed it two runs. A trimmed `restore` takes the run's opening time from the record's opening, which a witness does not hold.
- Before any history is trimmed, the fold must match `certify` and the independent oracle on every window shape.
- Links and checkpoints that an earlier build wrote still load.
- SRS invariant I1, I17 and the learning scope need only the certificate over every window and the event log, so no SRS line changes. Section 5 of the contract and its Checkpoint paragraph change with this record.
