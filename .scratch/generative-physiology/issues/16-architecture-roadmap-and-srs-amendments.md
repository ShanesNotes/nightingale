# 16: Generative architecture roadmap, reference building blocks and SRS amendment proposals

Type: research
Status: open
Blocked by: none

**Question:** Beyond the first experiment (`C4a` on the slice and [ticket 14](14-first-experiment-on-the-whole-body.md)'s meal experiment on the whole body), what staged architecture roadmap best guides generative models around Nightingale's engine? Which first-principles building blocks from working open-source references carry over, which SRS or governance guardrails stay hard, and which deserve explicit ADR or SRS amendments?

This ticket captures a multi-model exploration pass (read at `origin/main` `b74fad1`, including [native-contract 100](../../native-contract/issues/100-the-body-moves-after-admission-what-moves-and-why.md) from nightingale#229) so local `pstack` passes can turn settled forks into ADRs, build tickets and prototypes. Nothing here is decided yet; every architectural fork awaits `pstack architect`, `pstack interrogate` and clinician-lead sign-off.

## Acceptance (for the local `pstack` follow-up)

- [ ] Run `pstack architect` on the M2 warm-start identity fork ([native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md), [81](../../native-contract/issues/81-a-runs-identity-names-no-revision-of-the-engines-numerics.md)): separate proposal diagnostics from `RunIdentity` and draft the ADR
- [ ] Run `pstack architect` on provenance for calibrated, distilled and observer models (SRS rules 12 and 16, [record 0003](../../../docs/adr/0003-inferred-parameters.md)) and draft the ADR
- [ ] Run `pstack architect` on the verified mid-run entry and grouped-progression forks (SRS rules 14, I4, I17, I18) before M5 or M6 design begins
- [ ] Add a fast research path rule in [`tools/gate/paths.toml`](../../../tools/gate/paths.toml) for `tools/generative-physiology/**` so Python and driver research edits do not trigger the Rust engine's full mutation sweep
- [ ] Prototype the two non-blocked early wedges while M0 finishes: (a) M2 non-ML nearest-neighbor lookup for admission warm-starts, and (b) M4 rule-based ECGSYN waveform observer on short runs with a blinded clinician look test
- [ ] Update [ticket 14](14-first-experiment-on-the-whole-body.md) and [ticket 15](15-learned-emulator-for-parameters.md) with the settled choices from sections 1 to 4 below once M0 resolves

---

## 1. Merged staged roadmap (M0 to M6)

Synthesized from two independent passes: first-principles code dissections of four working open-source references (section 2) and a milestone-gated teacher-student proposal inspired by rapid iterative hardware development and Tesla's offline-world-simulator to tiny-edge-student split. Each stage starts with the cheapest non-ML baseline that can pass its pre-written exit gate.

| Stage | Goal | Exit gate (written before work starts) | Starting baseline, then ML upgrade | Unblocks / depends on |
|---|---|---|---|---|
| **M0: Engine stable** | Whole body runs 48 simulated hours with no certificate refusal ([body 28](../../body/issues/28-two-synthetic-days.md), [native-contract 100](../../native-contract/issues/100-the-body-moves-after-admission-what-moves-and-why.md)); oscillation damping from backward Euler fixed ([native-contract 91](../../native-contract/issues/91-the-accuracy-gate-holds-no-oscillation.md)). | Fixed suite reaches 48 simulated hours with no refusal; drift measured against reference; oscillation amplitude check passes. | Mechanistic Rust engine (`Core`). | Hard prerequisite for training final M3, M5 and M6 weights. Does not block M2 lookup or M4 rule-based prototypes. |
| **M1: Replayable teacher harness** | Generate version-tagged synthetic patients and **paired counterfactual intervention branches** (same start state, intervention applied vs. withheld) with streaming reduction capture. | Every sample replays bit-for-bit from its tags; refused runs are kept and labeled, never silently dropped; disk stays bounded on multi-hour runs. | Space-filling trait sampling + paired counterfactual schedules ([ticket 14](14-first-experiment-on-the-whole-body.md) meal branches). | Feeds M2 through M6. |
| **M2: Admission seeding** | Warm-start `admit` ([native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md)) so new patients converge faster while `admit` remains the sole exact judge. | On held-out synthetic patients, seeded and cold `admit` converge to the same steady state within tolerance, and median wall time drops. | **Non-ML first:** nearest-neighbor lookup in the M1 admitted library. Add an amortized neural seeder only if lookup leaves large speed on the table. | Can prototype now on short runs after settling `RunIdentity` (section 3). |
| **M3: Parameter emulator** | Predict engine steady and dynamic response summaries from traits in milliseconds with calibrated error bars ([ticket 15](15-learned-emulator-for-parameters.md)). | On held-out engine runs, 90% intervals contain true engine outputs about 90% of the time; emulator proposes candidates only, and the engine verifies survivors. | **GP first** for low-dimensional summaries, benchmarked head-to-head against **`sbi` sequential Flow Matching (`NPE_C` / `FMPE`)** as trait dimension and coupled dynamics grow. | Accelerates packet calibration and cohort design. |
| **M4: Read-only observer stack** | Turn accepted state into monitor waveforms (ECG, arterial pressure, pleth, capnography), physical exam findings and realistic imperfect charts (missing labs, delayed notes, cuff noise). | Observers cannot write state (enforced in code, I7); every output is checked against underlying state; blinded clinician look test passes. | **Rule-based first:** ECGSYN-style shapes locked to engine rhythm phase; add conditional generative residual only for artifacts and realism rules miss (loose lead, motion, cold-hand PPG dropout). | **Must precede M5** so chart inference trains on realistic imperfect charts, not pristine state. |
| **M5: Chart inference and belief debrief** | From an imperfect bedside chart alone, infer a calibrated probability cloud (posterior) over consistent hidden traits and start states ([ideas](../../ideas.md), chart-to-engine). Powers `admit` warm-starts and clinician debriefs in [`clinical-workspace/`](../../clinical-workspace/map.md). | Simulation-based calibration (`SBC` / `TARP`): true hidden traits land inside the 90% posterior cloud ~90% of the time on engine-made charts; MMD misspecification test catches out-of-support patients. | `sbi` (`FMPE` / `NPE`) with a NaN-tolerant set encoder (`PermutationInvariantEmbedding`) for irregular sparse charts. | Needs M1 data, M2 seeding path and M4 imperfect charts. |
| **M6: Standalone conserved student with fallback** | Compact, fast edge student distilled from the engine for interactive encounters and multi-patient wards, with live support detection and exact-engine fallback. | Matches engine within tolerance on clinician-visible signals over long free runs, passes out-of-policy and closed-loop counterfactual tests, and catches planted OOD cases to trigger fallback. | Named conserved totals + `CompiledBody` causal dependency mask + continuous-time two-ended transfers/reactions. Optional INT8 quantization on rate weights while keeping the ledger strictly `float64`. | Needs M0 (`body 28` + `nc91`), M1 counterfactuals and verified fallback protocol (checked in TLA+). |

### Parked north-star work (do not build before M6 earns it)

- **Site-class graph neural ODE (`MeshGraphNets` / port-Hamiltonian style):** learn exchange laws per site class over the `Map` graph so refining a grouped region transfers without retraining from scratch.
- **Parallel-in-time (Parareal) with a learned coarse propagator:** parked because Parareal is known to go unstable on wave-like oscillatory systems (pulsatile beats and valve crossings) and requires phase-matched window starts.
- **One-step flow distillation (`MeanFlow`):** candidate for ultra-fast edge decoders once M4/M5/M6 multi-step flow models prove their value.
- **Learning past the teacher:** small rate-only corrections checked against independent published human studies (planted-discrepancy line from [ticket 01](01-concept-brief.md)).

### Clinician-lead judgment gates (cannot be delegated to models)

1. **M0:** which slow 48-hour drift is visible at the bedside versus clinically negligible.
2. **M2 and M5:** what a real admission chart actually contains, misses or records late.
3. **M4:** blinded look test on rendered waveforms, artifacts, exam cues and chart realism.
4. **M5:** which gaps between learner belief and hidden physiological reserve are worth teaching in debrief.
5. **M6:** the exact list of clinician-visible signals and counterfactual clinical behaviors that set the student's pass bar.

---

## 2. First-principles building blocks extracted from working reference codebases

Four open-source codebases were inspected directly from source (outside this repo) to identify reusable mechanisms and traps to reject.

### A. `bobjz/H2NCM` (Hybrid Neural ODE Causal Modeling, insulin-glucose)

- **Keep: causal wiring without hard-coded equations (`MNODE_model.py`, `DAG_RNN`).** Each state's small network reads only its physiological parents rather than every total in the body. On Nightingale's 13-system whole body, derive allowed input totals directly from `CompiledBody`'s structural dependency graph ([solver contract](../../../docs/architecture/solver-contract.md)) instead of dense all-to-all MLPs as in slice `C4a` ([ticket 08](08-model-forms.md)).
- **Keep: two-stage residual training (`LPSC_reduced_model.py`, `turn_on_closure`).** Train the structured named model first, freeze it, then train a small residual only on leftovers. In Nightingale, restrict any Stage-2 residual to **modulating transfer rates only**, never adding directly to a pool.
- **Keep: paired counterfactual contrast loss (`utils.py`, `synthetic_experiments/generate_synthetic_data.py`).** Explicitly supervise intervention contrasts from identical start states so collinear confounding (e.g., illness severity and norepinephrine, or carbs and bolus insulin) cannot invert cause and effect.
- **Reject:** fixed 5-minute Euler stepping (`Gp + DGp`) and unconstrained per-pool neural additions that break mass conservation (and contain a copy-paste bug at `LPSC_reduced_model.py:105`).

### B. `DrunkJin/dreamer-from-scratch` (PyTorch DreamerV3 world model)

- **Keep: `observe_step` vs. `imagine_step` separation (`dreamer/rssm.py`).** Assimilating observations into belief (`observe`) stays strictly distinct from rolling forward without readings under interventions (`imagine`). A standalone virtual patient compiles only `imagine` (enforcing I7); a workspace agent uses `observe` on chart entries and `imagine` for counterfactual previews.
- **Keep: continuous state plus stabilized discrete modes (`rssm.py`, `unimix`).** Continuous pools and memories paired with discrete regime indicators once later scope stages add acute mode transitions.
- **Keep: `symlog` + `twohot` scaling (`dreamer/utils.py`).** Handles 10+ orders of magnitude across body water (~42,000 mL) and nanomolar transmitters alongside Nightingale's `Natural size` ([GLOSSARY.md](../../../GLOSSARY.md)), and yields distributional observer outputs.
- **Keep: rare-event upweighting (`dreamer/networks.py`, `ContinueHead`).** Upweight rare terminal or acute-crisis transitions so the model does not lazily predict indefinite stability (SRS rule 7).
- **Reject:** opaque black-box `GRUCell` state with no conservation, fixed discrete ticks and RL reward/actor loops inside the patient model.

### C. `sbi-dev/sbi` (Simulation-Based Inference in PyTorch)

- **Keep: amortized (`FMPE`) and sequential multi-round (`NPE_C`) inference (`sbi/inference/trainers/`).** Sequential rounds focus expensive Rust runs on plausible parameter regions (10x to 100x fewer simulations for [ticket 15](15-learned-emulator-for-parameters.md)); amortized flow matching runs in milliseconds per new chart for M2/M5 and represents multi-modal clinical causes (e.g., hypovolemia vs. vasoplegia at the same MAP of 65).
- **Keep: NaN-tolerant set encoders (`PermutationInvariantEmbedding`, `accepts_nan_input = True`).** Ingests irregular, sparse, unordered bedside charts without fabricating imputed labs.
- **Keep: formal calibration diagnostics (`sbc.py`, `tarp.py`).** Quantitative pass gates proving whether a claimed 90% posterior cloud actually contains the true hidden patient 90% of the time.
- **Keep: MMD model-misspecification alarm (`misspecification.py`, `calculate_p_misspecification`) + `EnsemblePosterior`.** Statistical support test asking whether an observed patient could even come from the simulator, directly supplying [ticket 01](01-concept-brief.md)'s required support signal.

### D. `beards-lab/PhysicalAI` (Cardiovascular digital twin + generative virtual cohort)

- **Keep: correlation-preserving cohort generation + physics rejection filter (`VirtualDT_correlated`, `VirtualDT_phys`).** Sample traits with realistic joint covariance rather than independently, and filter every candidate through the exact mechanistic engine (`admit` + certificate) before downstream training.
- **Reject:** MATLAB dependency, KNN lab imputation (masks clinical missingness) and static binary classifiers over snapshot tables.

### Remaining reference dissections queued for local follow-up

- `google-research/neuralgcm` (physics core + learned tendencies) when refining M6's student boundary.
- `ECGSYN` (PhysioNet) and `SSSD-ECG` when prototyping M4's waveform observer stack.
- `FreedomIntelligence/SepsisAgent` (propose-simulate-refine boundary, rejecting retrospective-only confounded training) when wiring M5/M6 into [`clinical-workspace/`](../../clinical-workspace/map.md).

---

## 3. First-principles audit: guardrails to keep hard vs. SRS and governance amendments to propose

Reasoning from clinical and physical first principles rather than treating Stage-1 documentation as frozen:

| Area | Stance | First-principles reason | Proposed path (`pstack` + ADR) |
|---|---|---|---|
| **State vs. observation vs. chart separation; read-only observers** ([SRS rules 1, 13](../../../docs/srs.md#3-hard-rules)) | **Keep hard** | A monitor artifact or charting mistake never alters underlying physiology; merging them ruins clinical honesty. | Enforce in types and code across both engine and generative models. |
| **Exact conservation on body state** ([SRS rule 3, I10](../../../docs/srs.md#3-hard-rules)) | **Keep hard on ledger; allow quantized rate weights** | Exact balance eliminates whole classes of unphysical hallucinations. Quantizing neural *rate weights* to INT8 (Tesla-style edge speed) is safe if the *state and ledger update* (`amounts += B @ extents`) remain exact `float64`. | Split precision cleanly in M6: INT8/FP16 rate predictors + exact `float64` conserved ledger. |
| **Value provenance for fitted and distilled models** ([SRS rules 12, 16](../../../docs/srs.md#3-hard-rules), [record 0003](../../../docs/adr/0003-inferred-parameters.md)) | **Challenge and amend** | Rules 12 and 16 were written for static packet numbers. Treating `sbi`-calibrated parameters, M6 student weights or M4 artifact observers as "no-source inference records" is a category error. | Propose an ADR defining explicit provenance tiers: `calibrated-to-response` (engine-verified against sourced dynamics), `distilled-from-engine` (teacher hash + `SBC`/`TARP` + OOD gate) and `learned-observer` (read-only + clinician look test). |
| **Mid-run chronic and subacute disease progression** ([SRS rule 14, I15](../../../docs/srs.md#3-hard-rules)) | **Challenge and amend** | Real ward and ICU patients progress across shifts (vasoplegia, ARDS stiffness, AKI) long before molecular cellular-injury laws exist for all 13 systems. Silent unlogged parameter nudges break replay, so mid-run progression must stay deterministic and logged. | Propose an ADR for **grouped progression events** (temporal analog of [record 0002](../../../docs/adr/0002-map-starts-grouped.md)): explicit, timestamped, deterministic parameter-shift events in the append-only event log, labeled as unmodeled mechanisms to retire when damage-integral laws land. |
| **Verified non-rest state entry (chart start and M6 fallback)** ([SRS I4, I17, I18](../../../docs/srs.md#4-invariants-each-becomes-a-test)) | **Challenge and amend** | Forbidding unverified state writes protects `PatientRecord`, but both M5 (starting an ICU patient from a chart) and M6 (handing a mid-encounter student back to the Rust engine) need a legal entry seam. | Propose an ADR + TLA+ statechart (`pstack statechart`) comparing **Event-Log Replay Fallback** (exact single-ledger replay from admission/checkpoint) against a **Verified Admit-at-Condition Branch Seam** (engine verifies conservation, speciation and stability before opening a tagged `Branch`). |
| **Warm-start `RunIdentity`** ([native-contract 81](../../native-contract/issues/81-a-runs-identity-names-no-revision-of-the-engines-numerics.md), [86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md)) | **Challenge and clarify** | Replay should depend on the verified admitted state and solver contract, not which M2 lookup or neural seeder sped up Newton convergence. | Propose an ADR keeping proposal provenance in diagnostics and out of `RunIdentity`. |
| **Research tooling gate scope** ([`tools/gate/paths.toml`](../../../tools/gate/paths.toml)) | **Challenge and amend** | Running Rust mutation testing (`40-mutants`) on Python research edits under `tools/generative-physiology/**` slows iteration with zero engine-safety benefit. | Add a dedicated path rule in `tools/gate/paths.toml` running `selftest.py` and driver tests without `40-mutants`. |

---

## 4. Open architectural questions for `pstack architect` and `pstack interrogate`

1. **Whole-body causal masking (M6 / ticket 14):** Compare dense `C4a` against `CompiledBody`-masked transfer MLPs (`H2NCM`'s `DAG_RNN` pattern) on the whole-body meal experiment.
2. **M3 bake-off (`ticket 15`):** At what trait dimensionality and dynamic response complexity does `sbi` sequential Flow Matching (`NPE_C` / `FMPE`) overtake a Gaussian Process emulator?
3. **M6 fallback continuity:** In TLA+ (`docs/statecharts/`), prove that neither Event-Log Replay Fallback nor Verified Branch escalation can drop an intervention, double-apply a delivery or leak across the causal fence (I5).
4. **M4 artifact truth-checking:** Define concrete automated checks ensuring a generative waveform artifact (e.g., motion noise or lead dropout) never contradicts underlying rhythm, pressure or saturation state without being tagged as artifact.

## Non-goals

- Changing `docs/srs.md`, `docs/adr/`, `crates/` or `tools/` in this research ticket.
- Training models on the whole body before M0 (`body 28` and `native-contract 91`) passes.
- Claiming clinical validity from synthetic runs or look tests alone.
