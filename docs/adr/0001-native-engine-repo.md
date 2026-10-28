# The native engine lives in its own repo, and nightingale-pulse is a read-only reference

The native Rust engine (the SRS, the atlas and the solver contract) moved out of the repo that holds the Pulse port, the playground, the kernel, the chart and the UX work. That repo was renamed ShanesNotes/nightingale-pulse, and this repo imported its files at [bdc01c567f6cadacbd96728c0600a2e052882a59](https://github.com/ShanesNotes/nightingale-pulse/commit/bdc01c567f6cadacbd96728c0600a2e052882a59). This repo never depends on pulse to build or run and reads pulse files only at that commit, so the engine stays free of the Pulse path and a Pulse diff stays a separate claim (SRS rule 6).

## Considered options

- One repo with a native subtree: rejected, because pulse's README, roadmap, lane docs and records contradict the SRS and keep steering agents.
- A submodule, or a pin manifest the gate checks: rejected, because the import commit on pulse's protected main cannot change, so a permalink at it already pins, and a gate that reads pulse would need a pulse clone in every seat.

## Consequences

- Links and issue numbers written before the split to ShanesNotes/nightingale mean nightingale-pulse. Write nightingale-pulse#N or nightingale#N, never a bare #N.
- Decision records here are one series from 0001. A pulse record is cited as pulse 000N, or pulse ADR-NN, with a permalink at the import commit, and is never copied. One that still matters is written fresh as a new record here.
- Files imported from pulse cite its records as pulse 000N from the link commit on, so a bare number here always means a record in this repo.
