# 04: First expect-style physiology trace

Type: task
Status: ready-for-agent
Blocked by: `.scratch/native-contract/issues/10-slice-laws.md`

A trace test runs a short deterministic slice and keeps its printed trace inside the test, so a PR diff shows a behavior change next to the code change. A trace shows that behavior changed, never that it is right: only checks drawn from anchors and requirements judge physiology (AGENTS.md, Behavior).

- **Tool:** `expect-test` 1.5.1 as a dev-dependency. The expected text is an inline `expect![[...]]` literal; `UPDATE_EXPECT=1 cargo test` rewrites it, and a failure prints Expect beside Actual. Chosen over `insta`, which keeps snapshots in separate files.
- **Format:** fixed seed, simulated clock and a synthetic fixture, so nothing is host-dependent and nothing is a clinical claim. At most 40 rows of 100 characters. A header gives each column its unit from its type (`t_s MAP_mmHg V_blood_mL HR_bpm urine_mL_min`). Fixed precision per quantity. No line starts with whitespace, because expect-test strips indentation. Events get their own line (`# t=12.0 s refused: StageResidual`). Waveforms are a text strip, one row of digits 0 to 9. A changed trace is explained in one PR sentence.
- **Runs:** `tools/verify --changed` runs a trace when the crate that composes its slice changes, the merge run runs it on any change in its closure, and `tools/verify --full` always does. The trace registers as `smoke` in its crate's `tools/gate/paths.toml` row, which admits `expect-test` as `dev_external = { expect-test = "=1.5.1" }`. A trace over 30 s is an ignored test with its reason, so only the full run takes it.
- **First one:** `minimal_slice_60s` in `crates/ng-validate/tests/trace.rs`, in the PR that first composes pulsatile CV, FL, RN and the NS baroreflex. 60 simulated seconds at one row per 5 s, ending with `# checkpoint round trip: identical`. It doubles as the gate's smoke test. Each arena seat carries its own trace, and the pick PR keeps the winner's.
