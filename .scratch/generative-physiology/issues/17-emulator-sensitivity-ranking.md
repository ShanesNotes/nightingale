# 17: Stage A sensitivity ranking and the constant-classification audit

Type: research
Status: open
Blocked by: 15, `.scratch/native-contract/issues/95-the-record-trims-its-history.md`

**Question:** Partition all 558 rung 0 constants in [research 16](../../research/issues/16-constants-and-what-binds-them.md) into [ticket 15](15-learned-emulator-for-parameters.md)'s four mutually exclusive buckets, isolate an unblocked Stage A sensitivity-ranking design from the blocked Stage B history-matching fit, and specify grouped inputs, response summaries, hold-out protections, `L9` refusal accounting, parameterized run budgets, and falsifier tests.

This ticket holds a verified audit of research 16's rows and a draft Stage A design for [ticket 15](15-learned-emulator-for-parameters.md) and [ticket 16](16-architecture-foundations.md) (sections 2, 5.3, 6, and 7). It is open again. A review recounted the audit and it reproduces exactly (section 1). The same review found that Stage A is not unblocked, and section 2 says what it waits on. Stage A sets no packet value, so SRS rule 16 and record 0003 are untouched by it. Stage B (fitting any parameter to sourced responses) stays blocked on [research 05](../../research/issues/05-parameters.md), [body 30](../../body/issues/30-step-plans-on-the-whole-body.md), and the SRS decision record.

## Draft answer

Parsing all **558 rows** (`CV` through `IM`) of [research 16](../../research/issues/16-constants-and-what-binds-them.md) at `origin/main` (`ffcd1a7a`) reconciles cleanly with its summary table and partitions into [ticket 15](15-learned-emulator-for-parameters.md)'s four mutually exclusive buckets as:

1. **Measured by a source:** **409** rows (`411` `sourced` minus `2` admission-calibrated rows).
2. **Derived by an identity:** **32** rows (`32` `derivation`).
3. **Pinned by an anchor at admission:** **2** rows (`CV-066` and `RN-016` per [research 05](../../research/issues/05-parameters.md#L48)).
4. **Candidate for response-only pinning or sensitivity ranking:** **115** rows (`38` `inference` + `57` `model default` + `20` `unknown`).

Inside Bucket 4 (`115`), **51** `model default` constants are fixed numerical, unit, gauge, or structural choices and **12** `unknown` constants are gauge or series wall-transfer partners inside inseparable sets. Removing those `63` fixed/collapsed choices and `3` deferred missing-receptor/missing-form constants (`CV-086`, `CV-087`, `MS-003`) leaves **49** Bucket 4 rows. Only **13** of them have a key a crate reads at `9a7469ff`, and **36** read "no key yet" in research 16 (section 4). Once every law is built, the 49 group into **`d = 46` active Bucket 4 axes** by section 4's own steps (or **`d = 77` Tier-1 Stage A axes** when adding grouped `none named` and `at-the-instant` Bucket 1 priorities for [research 05](../../research/issues/05-parameters.md)). The first draft said 44 and 75, which its steps do not give.

- **Model default for Stage A:** **Shape B** (direct scrambled Sobol / maximin Latin Hypercube space-filling design of `N_B = 6d + 20` runs, binary `L9` GP-classification feasibility surface, and per-summary GP-ARD with Shapley/Saltelli variance decomposition). At `d = 46` (`N_B = 296` 6 h runs), Stage A requires `726.8` worker-hours at `2.5x` real time (`w = 0.40`), `1,792.4` at `1x` (`w = 1.00`), or `194.0` at `10x` (`w = 0.10`) for single runs, reusing every run into Stage B Wave 1 and surviving non-rectangular `admit` holes. The meal rows need a control run beside each run, which multiplies those hours by `1.65` to `1.75`, and each run writes `276 GB` of links until native-contract 95 lands (section 7).
- **One-line alternative:** **Shape A** (two-step screening: `r(d + 1)` Morris elementary-effect trajectories followed by `10 d_act + n_val` active-subset GP-ARD + Sobol runs).
- **Concrete switch trigger:** Switch to Shape A if `d > 80` axes are screened simultaneously, if [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) measures `w > 0.8 s/sim-s` (`< 1.25x` real time) requiring a short-horizon (`1,800 s`) pre-screen, or if initial admission refusals exceed `50%` so local trajectories from admitted seeds are needed to stay in the feasible region.

## 1. Machine-verified 558-row constant audit

### Reconciliation with `research 16` at `origin/main` (`ffcd1a7a`)

A read-only parser over every `| XX-NNN |` row in [research 16](../../research/issues/16-constants-and-what-binds-them.md) matches the summary table (`lines 352-367`) cell for cell: `411` `sourced`, `32` `derivation`, `38` `inference`, `57` `model default`, `20` `unknown`, `0` `none`, summing to **558** rows across the 13 systems (`IM` holds `0` rows). Cross-cutting summary counts also reconcile exactly: `28` `at-the-instant` rows (`yes` or `yes: ...`), `116` rows with no work named (`115` `none named` plus `NS-063` `None named`), and `231` rows reading an external unheld quantity (`232` containing `no row holds` minus `CV-054`, which marks standard gravity `g_n` as physical constant `X-K01`).

One commit-drift note applies to crate keys (`line 367` of `research 16`): `152` rows now carry a backticked `ParameterKey` present in `crates/` on `origin/main` (`ffcd1a7a`), comprising the `141` non-`MT` slice keys cited in `line 367` plus `11` `MT` thermal keys (`MT-016`, `MT-018`, `MT-024` to `MT-028`, `MT-030` to `MT-033`) added in `nightingale#226`, alongside `7` `MT` rows folded into composite keys (`MT-013` to `MT-015`, `MT-020` to `MT-023`), `2` zero-gauge rows without keys (`MT-017`, `MT-019`), and `397` rows marked `no key yet`.

### Four mutually exclusive buckets by system

Every row maps into exactly one bucket from [ticket 15](15-learned-emulator-for-parameters.md) (`B1 + B2 + B3 + B4 = Total = 558`). Bucket 4 is further stratified into fixed `model default` constants (`MD fix`, `51`), collapsed `unknown` convention/series choices (`Unk col`, `12`), and uncollapsed Bucket 4 candidates (`B4 act`, `52`, of which `3` await a receptor or law form; of the other `49`, `13` have a key a crate reads and `36` read "no key yet", section 4).

| Sys | Total | B1: Sourced | B2: Derived | B3: Admit | B4: Candidate | Inf | MD | Unk | MD fix | Unk col | B4 act | Instant | No work | Spends check |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `CV` | 87 | 55 | 13 | 1 | 18 | 3 | 12 | 3 | 10 | 0 | 8 | 2 | 27 | 0 |
| `FL` | 28 | 21 | 3 | 0 | 4 | 2 | 2 | 0 | 2 | 0 | 2 | 0 | 3 | 0 |
| `RN` | 85 | 46 | 3 | 1 | 35 | 20 | 11 | 4 | 10 | 2 | 23 | 5 | 29 | 2 |
| `NS` | 63 | 51 | 0 | 0 | 12 | 4 | 6 | 2 | 5 | 2 | 5 | 14 | 7 | 9 |
| `RS` | 15 | 9 | 6 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 4 |
| `BC` | 38 | 35 | 1 | 0 | 2 | 0 | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 0 |
| `EN` | 54 | 47 | 1 | 0 | 6 | 1 | 4 | 1 | 4 | 1 | 1 | 2 | 15 | 1 |
| `HP` | 39 | 27 | 0 | 0 | 12 | 4 | 3 | 5 | 3 | 5 | 4 | 0 | 8 | 0 |
| `GI` | 39 | 35 | 0 | 0 | 4 | 1 | 2 | 1 | 2 | 0 | 2 | 5 | 3 | 0 |
| `HC` | 6 | 4 | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 1 |
| `MT` | 81 | 59 | 4 | 0 | 18 | 2 | 14 | 2 | 12 | 1 | 5 | 0 | 15 | 2 |
| `MS` | 23 | 20 | 0 | 0 | 3 | 0 | 2 | 1 | 2 | 0 | 1 | 0 | 8 | 0 |
| `IM` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| **All** | **558** | **409** | **32** | **2** | **115** | **38** | **57** | **20** | **51** | **12** | **52** | **28** | **116** | **19** |

**Reconciliation on Bucket 3 (`pinned by an anchor at admission`: `2` rows):**
- [Research 05](../../research/issues/05-parameters.md#L48) (oversight ruling 2) and [research 16](../../research/issues/16-constants-and-what-binds-them.md#L435) assign **only `CV-066`** (`cv.resistance`, `systemic arteries` to `glomerular bed`) and **`RN-016`** (`rn.sodium-capacity`, `renal tubule` to `peritubular bed`) to admission calibration against the mean arterial pressure and filtration anchors, keeping their sourced measurements solely as admissible population bounds.
- Synthetic code divergence: `crates/ng-validate/src/slice.rs:729-734` and `crates/ng-validate/src/body/packet.rs:634-639` also call `ParameterPacket::calibrated` on `cv.intrinsic-rate` (`CV-036`) for the synthetic `heart rate` anchor (`3` calibrated keys in synthetic code). In the clinical dataset, [packet-reference-man.jsonl](../../../data/packet-reference-man.jsonl#L23) already holds `ns.intrinsic_rate.sinoatrial` (`CV-036`) from Katona 1982 as `sourced`, [closure.jsonl](../../../data/closure.jsonl#L6) (`XC-36`) checks `R0 * m * n` against `cv.hr.supine_rest`, and [research 05](../../research/issues/05-parameters.md#L20-L23) leaves `CV-036` sourced (`B1`) and `CV-039`/`CV-040` derived (`B2`).

### Explicit accounting of special groups and cross-cutting flags

1. **`inference` (`38` rows, all in Bucket 4):**
   - `CV` (`3`): `CV-049`, `CV-050`, `CV-056`.
   - `FL` (`2`): `FL-022`, `FL-023`.
   - `RN` (`20`): `RN-007`, `RN-008`, `RN-009`, `RN-010`, `RN-024`, `RN-025`, `RN-032`, `RN-033`, `RN-034`, `RN-037`, `RN-038`, `RN-040`, `RN-041`, `RN-042`, `RN-046`, `RN-047`, `RN-048`, `RN-053`, `RN-055`, `RN-071`.
   - `NS` (`4`): `NS-024`, `NS-025`, `NS-026`, `NS-027`.
   - `EN` (`1`): `EN-023`.
   - `HP` (`4`): `HP-032`, `HP-033`, `HP-034`, `HP-035`.
   - `GI` (`1`): `GI-005`.
   - `HC` (`1`): `HC-005`.
   - `MT` (`2`): `MT-001`, `MT-010`.
   - Note: `4` are `at-the-instant` (`RN-040`, `RN-041`, `RN-042`, `RN-053`); `2` spend a check (`MT-001`, `RN-040`); `2` are receptor site counts (`RN-071`, `HP-032`) that cancel out of occupancy unless binding depletes ligand.

2. **`model default` (`57` rows, all in Bucket 4):**
   - **Fixed numerical smoothing widths (`25`):** `CV-058`, `CV-063`, `FL-011`, `RN-028`, `RN-056`, `RN-060`, `RN-065`, `RN-067`, `RN-076`, `RN-081`, `NS-030`, `NS-033`, `NS-055`, `NS-056`, `NS-063`, `EN-009`, `EN-026`, `EN-030`, `HP-009`, `HP-025`, `GI-027`, `MT-011`, `MT-025`, `MT-029`, `MS-007`.
   - **Fixed positivity/guard floors and pass-throughs (`7`):** `CV-051` (`R_read`), `CV-052` (`dp_open`), `RN-020` (`K_cl`), `HP-001` (`P_L`), `MT-045` (`K_ii` brain), `MT-047` (`K_ii` other beds), `MT-060` (`K_T`).
   - **Fixed unit factors, declared references, heat gauges, and inseparable normalizations (`15`):** `CV-031` (`a_peak`), `CV-035` (`phi_c`), `CV-057` (`P_ref`), `CV-062` (`1/2`), `RN-021` (`f0`), `RN-022` (`h`), `BC-032` (`c_0`), `EN-011` (`G_b`), `GI-004` (caloric weights), `MT-012` (oxygen debt reference), `MT-016` to `MT-019` (`T_ref`/`H_ref`), `MT-067` (`a_i` glucose weight).
   - **Fixed structural, reflection, anatomy, or test-hazard choices (`4`):** `CV-060` (collapse exponent), `CV-079` (`chi` feedback share), `FL-021` (`sigma`), `MS-001` (`lambda_stir`).
   - **Active unmeasured physiological rates/gains (`6`):** `CV-080` (`S_d`), `CV-081` (`K_d`), `RN-079` (`k_V`), `NS-053` (`k_set`), `MT-043` (bed heat transfer speed), `MT-076` (`k_b`). Only these `6` of `57` enter Stage A physiological screening; the `51` fixed choices are held constant (with an optional numerical perturbation check on the `25` widths and `7` floors).

3. **`unknown` (`20` rows, all in Bucket 4):**
   - **Collapsed convention or series wall-transfer partners in inseparable sets (`12`):** `BC-034`, `EN-025`, `HP-002`, `HP-003`, `HP-004`, `HP-005`, `HP-006`, `MT-063`, `NS-058`, `NS-061`, `RN-051`, `RN-054`.
   - **Titration/absorption splays (`3`, uncertain measurable splay vs numerical width):** `RN-070`, `RN-078`, `GI-038`.
   - **Active unmeasured or missing-form constants (`5`):** `CV-006` (pulmonary venous compliance), `MT-008` (`k_t`), plus `3` missing-receptor/missing-form deferred constants (`CV-086`, `CV-087`, `MS-003`).

4. **`at-the-instant` (`28` rows across 3 routes):**
   - **`23` in Bucket 1 (`sourced`):** `CV-083`, `CV-084`, `NS-017`, `NS-018`, `NS-020`, `NS-021`, `NS-022`, `NS-023`, `NS-028`, `NS-029`, `NS-031`, `NS-032`, `NS-034`, `NS-035`, `NS-036`, `NS-062`, `EN-028`, `EN-029`, `GI-002`, `GI-003`, `GI-033`, `GI-034`, `GI-035`.
   - **`4` in Bucket 4 (`inference`):** `RN-040`, `RN-041`, `RN-042`, `RN-053`.
   - **`1` in Bucket 4 (`unknown`):** `RN-054` (collapsed convention partner of `RN-053`).
   - Although the `23` `sourced` rows sit in Bucket 1, their source models fitted mean inputs whereas the engine evaluates at the instant, so non-check-spending subsets among them are included in Stage A sensitivity ranking to measure mismatch risk.

5. **`sourced` rows with `none named` (`44` of `116` unworked rows; remaining `72` sit in B2/B4):**
   - `31` receptor-binding constants (`RN-001`..`003`, `RN-072`..`074`, `RN-082`..`085`, `EN-044`..`054`, `HP-038`..`039`, `MT-077`..`081`, `MS-021`..`023`).
   - `3` duplicate or retiring stand-in keys (`FL-008` == `CV-061`, `RN-006` == `FL-002`, `GI-028` retiring stand-in).
   - `10` non-receptor sourced constants (`CV-005`, `CV-008`, `CV-010`, `CV-034`, `CV-061`, `FL-004`, `GI-001`, `MT-021`, `MS-017`, `MS-018`).

6. **Inseparable parameter groups (`research 16` lines 385-397 plus flagged pairs):**
   - Every inseparable set reduces to one identifiable composite axis after fixing its convention/gauge member per [research 05](../../research/issues/05-parameters.md#L49):

| Group | Row IDs | Invariant combination | Convention-fixed / collapsed | Active Stage A axis |
|---|---|---|---|---|
| Aldosterone slope | `EN-021`, `EN-025` | `R0 / dK` | `EN-025` (`dK`, `unk`) | `R0` slope (`EN-021`, `B1`) |
| Vasopressin slope | `EN-032`, `NS-058` | `R1 / dP` | `NS-058` (`dP`, `unk`) | `R1` slope (`EN-032`, `B1`) |
| Vasopressin intercept | `NS-057`, `NS-060`, `NS-061` | `theta0 - k_V V_ref` | `NS-061` (`V_ref`, `unk`) | `theta0` (`NS-057`), `k_V` (`NS-060`) |
| Potassium secretion | `RN-049`, `RN-051`, `RN-053`, `RN-054` | `J1 c_ref^-n (1 + S_q (q/q_ref - 1))` | `RN-051` (`c_ref`), `RN-054` (`q_ref`) | `J1` (`RN-049`), `S_q` (`RN-053`, `B4`) |
| Hemoglobin charge offset | `BC-034` | `a_Hb` + lumped anion start | `BC-034` (`a_Hb`, `unk`) | Closed at admission |
| Liver series transfers | `HP-002`..`006`, `HP-016`, `HP-018`, `HP-020`, `HP-022` | Series `P_X` and `k_X` | `HP-002`..`006` (`P_X`, `unk`) | `HP-016`, `HP-018`, `HP-020`, `HP-022` (`B1`) |
| Peripheral glycerol transfer | `MT-063` | `P_X` non-limiting | `MT-063` (`unk`) | None (`0` active) |
| Liver glycogen turnover | `HP-007`, `HP-011` | Net glycogen change | Net (`k_syn - k_brk`) + turnover | `1` grouped pair (`2 -> 1` or `2` orthogonal) |
| Core/skin thermal zero | `MT-016`..`MT-019` | `T_ref`, `H_ref` gauge | All `4` (`MD`) | `0` active |
| Antoine vapour pressure | `MT-030`, `MT-031` | `P_ref exp(a)` | `MT-030` (`P_ref`, `B1`) | `a` (`MT-031`, `B1`) |
| Tissue CO2 store line | `MT-001`, `MT-002`, `MT-003` | `PCO2_ref - N_ref / K_t` | `MT-003` (`N_ref`, `B2` derived) | `MT-001` (`K_t`, `B4`), `MT-002` (`B1`) |
| Chamber active elastance | `CV-031`, `CV-012`..`015` | `a_peak E_active` | `CV-031` (`a_peak`, `MD`) | `CV-012`..`015` (`B1`; `CV-014/015` 0 in slice) |
| Thermostat commands | `NS-037`..`NS-047` | `1/D_c`, `theta - e_s/D_s`, `T_s_ref`, `M` | `NS-046`/`NS-047` convention | `NS-037`..`045` grouped slopes & weights |
| Baroreflex stretch & widths | `CV-057`, `NS-017`, `018`, `020`, `022` | `P_ref` with widths; `c_c + c_a` vs ratio | `CV-057` (`P_ref`, `MD`) | `NS-017/018` ratio (`1`), `NS-020`, `NS-022` |
| Muscle protein balance | `MS-008`, `MS-009` | `S - B = k_crn P_M` at rest | `MS-009` derived from `MS-008` | `1` turnover axis |

7. **`bindings that spend a check` (`19` rows from `research 16` line 437):**
   - **`14` in Bucket 1 (`sourced`):** `MT-004` (spends anchor `MT-A01`), `RS-013` (spends `RS-A02`), `NS-028` and `NS-031` (spend check `RS-V04`), `NS-034` (spends `NS-A01`, `RS-V02`, and body ticket 11 check), `NS-047` to `NS-052` (6 rows; spend `MT-A03` and `MT-V01` unless bound to separate cohorts), `EN-040` (spends `EN-A02`), `HC-002` (spends `HC-A01`), `RN-063` (spends `BC-A01`).
   - **`3` in Bucket 2 (`derivation`):** `RS-003`, `RS-005`, `RS-006` (read `FRC`, spending `RS-A04` and `RS-V03`).
   - **`2` in Bucket 4 (`inference`):** `MT-001` (`K_t`, spends `BC-A06` if summed to whole-body CO2 capacity) and `RN-040` (`S_P`, must never be fitted to pressure-natriuresis check `RN-L04`).

## 2. Clean separation between Stage A and Stage B

| Aspect | Stage A: Sensitivity ranking (this ticket) | Stage B: History-matching fit (blocked) |
|---|---|---|
| Blocker status | **Blocked** on crate keys, varied packets with a range for each axis, and bounded link storage (below). Its design is on paper, and no run can start | **Blocked** on [research 05](../../research/issues/05-parameters.md), [body 30](../../body/issues/30-step-plans-on-the-whole-body.md), and SRS decision record |
| Artifact returned | Ranked list of parameters by variance share (`S_i`, `S_Ti`), `L9` feasibility boundary, and trait shortlist | Non-implausible parameter region (`I(x) < c`) and engine-verified candidate proposals |
| Packet value written? | **Never.** Produces zero parameter values | Proposes values verified by engine runs |
| Provenance record needed? | **None for a value.** No value enters `data/` or a reviewed packet. But its runs need varied packets, and where varied values come from is point 3 of the decision ticket 15 asks for | Needs SRS decision record settling rule 12, rule 16 / [record 0003](../../../docs/adr/0003-inferred-parameters.md), and `ParameterPacket` construction |
| Check-spending / hold-out rule | Diagnostic sensitivity over non-hold-out summaries; flags any check-spending coupling | **Hard firewall:** held-out targets and spent checks never enter `I(x)` |

**What Stage A waits on** (each read at `9a7469ff`, measured by grep or by arithmetic on figures the tickets state):

1. **Crate keys.** A constant can be varied only where a law reads it. Of the 49 rows, 13 have a key a crate reads now: `CV-006`, `CV-049`, `CV-050`, `NS-024` to `NS-027`, `RN-007` to `RN-010`, `RN-024` and `RN-025`. The other 36 read "no key yet" in research 16's own Key field. By law form, 28 of the 49 belong to forms whose build tickets are all open: 17 in research 14's forms (body 17 and 18), 10 in research 15's (body 19 to 22) and 1 in research 13's (body 13 to 16).
2. **Varied packets, with a range for each axis.** Each run is a packet of varied values. The engine offers four fixed builders (`body_packet` and three variants, `crates/ng-validate/src/body/packet.rs:559-594`) and refuses a second value for a bound key (`DuplicateParameter`, `crates/ng-contract/src/packet.rs:301-305`). The driver admits one fixed packet (`tools/generative-physiology/driver/src/teacher.rs:739`), and its run spec has no packet grammar. Ticket 15 puts "where varied values come from" inside the decision it asks of the engine's process, and this review filed the evidence there. No range exists for any axis: `data/inferences.jsonl` is empty, and research 05 has no box ticked. A range that is a clinical value enters only through review (SRS rule 12, record 0005).
3. **Bounded link storage.** A six-hour whole-body run writes about 276 GB of links ([ticket 16](16-architecture-foundations.md), section 4), and the worker that measured the links has 133 GB free (native-contract 95). One run does not fit until that ticket lands or the driver's store lets links go.

The meal summaries of section 5 also wait for the fuel loop of body tickets 13 to 16.

## 3. Stage A architecture: two shapes, model default, and switch triggers

Let `d` be the grouped active input dimension, `T_admit` the wall seconds per admission (`~200 s` at `body 26` / `native-contract 86`, falling toward `~55 s` if start proposals land), `S_run` the simulated seconds per trajectory (`21,600 s` for a 6 h meal/posture run), and `w` the wall seconds per simulated second (`w = 1 / speed`). One run takes `t_run(w) = T_admit + S_run * w` wall seconds when admitted. A refused draw skips the stepping and costs more than an admission (section 6).

### Shape A: Two-step screening (Morris elementary effects -> active GP-ARD + Sobol)

- **Mechanism:** Step 1 runs `r` randomized one-at-a-time (OAT) Morris trajectories across `d` grouped inputs over a short or full horizon (`N_1 = r(d + 1)` engine runs), ranking parameters by `mu*` and `sigma` and discarding inert axes (`mu*` below `feature floor` / `noise floor`). Step 2 runs a `maximin` Latin Hypercube / Sobol design of `N_2 = c * d_act + n_val` full 6 h runs on the active subset `d_act < d`, fitting one Matérn-5/2 GP-ARD surrogate per summary and extracting Saltelli `S_i` and `S_Ti` from the GP posterior.
- **Run count:** `N_A(d, d_act) = r(d + 1) + c * d_act + n_val` (at `r = 6, c = 10, n_val = 20`: `N_A = 6(d + 1) + 10 d_act + 20`).
- **Failure pattern on Nightingale:** OAT perturbations hit non-rectangular `admit` boundaries (`InitializationFailure`), invalidating both adjacent elementary effects on a broken trajectory (`2` wasted runs per refusal). Furthermore, Morris grid points are not space-filling and cannot be cleanly pooled into the Step 2 GP without inducing severe anisotropy, wasting `r(d + 1)` expensive multi-hour runs.

### Shape B: Direct space-filling Sobol/LHS -> `L9` feasibility classifier + GP-ARD Shapley/Saltelli

- **Mechanism:** Draw a single scrambled Sobol / maximin LHS design of `N_init = k * d` points across the `d` grouped axes (run in batches so an initial `4d` wave can already diagnose admission rates and top axes before spending `2d` refinement runs, plus `n_val = 20` held-out engine runs).
- **Two-layer surrogate on the unified dataset:**
  1. A **GP binary feasibility classifier (`L9`)** trained on every run label (`admitted_complete` vs `admit_refused` / `run_faulted`), mapping `P_feasible(x)` across the parameter hypercube.
  2. One **Matérn-5/2 GP-ARD regressor per scalar summary `y_m`** trained strictly on `admitted_complete` runs (`N_ok`), eliminating `NaN` corruption from refused starts.
- **Sensitivity extraction:** Rank inputs by inverse ARD lengthscale (`1 / l_i^2`) **and** variance-based Shapley / conditional Saltelli total-effect shares (`S_Ti`) computed solely on the GP posterior over the feasible domain (`P_feasible(x) >= 0.5`), reporting `S_Ti(y_m | feasible)` beside boundary sensitivity `S_Ti(P_feasible)`.
- **Run count:** `N_B(d) = k * d + n_val` (at `k = 6, n_val = 20`: `N_B = 6d + 20`).

### Decision and switch triggers

- **Model default:** **Shape B** (`N_B = 6d + 20`). Every admitted run feeds both sensitivity ranking and the future Stage B Wave 1 emulator with zero throwaway screening trajectories, and `L9` admission holes drop individual points without invalidating neighbouring trajectories.
- **One-line alternative:** **Shape A** (Morris OAT screening -> active-subset GP-ARD + Sobol).
- **Concrete switch triggers:**
  1. **Dimension trigger:** Switch to Shape A if `d > 80` axes must be ranked simultaneously before narrowing, where a `6d` GP-ARD design exceeds the single-batch run budget.
  2. **Speed trigger:** Switch to Shape A (with `S_short = 1,800 s` screening runs in Step 1) if [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) measures `w > 0.8 s/sim-s` (`< 1.25x` real time), where `6d` full 6 h runs exceed practical worker-hours.
  3. **Refusal trigger:** Switch to local Morris perturbations around admitted seeds (`body_packet`) if space-filling `admit` refusals exceed `50%` in the initial `2d` batch so global LHS covers too little feasible volume.
  4. **Surrogate trigger:** Switch from per-summary GP-ARD to sparse/additive GP or a small neural surrogate (`ticket 15` / `16` fork 5) if `N_ok > 2,000` or `M > 50` summaries.

## 4. Exact input grouping and dimensionality

Stage A reduces raw parameters systematically before designing runs:

1. **Structural and invariant exclusions (`100` rows removed from `558`):**
   - `2` Bucket 3 admission-calibrated (`CV-066`, `RN-016`).
   - `32` Bucket 2 identity-derived (`RS-011` is a produced quantity). They follow their parents by identity, and the sampler has to recompute them: no shipped packet calls `derived` (`crates/ng-contract/src/packet.rs:354-355`), so the engine moves none of them by itself. Three read one of the 49: `CV-023` reads `CV-006`, `RN-039` reads `RN-040` and `RN-042`, and `HC-004` reads `HC-005`.
   - `51` fixed `model default` constants (smoothing widths, guard floors, unit factors, heat gauge zeros, anatomy/test switches).
   - `12` collapsed `unknown` convention or series wall-transfer constants inside inseparable pairs.
   - `3` missing-receptor/missing-form deferred constants (`CV-086`, `CV-087`, `MS-003`) that have no compiled slot in rung 0 yet.
   - Remaining raw pool: **`458` rows** (`409` Bucket 1 + `49` Bucket 4, of which `13` have a crate key now).

2. **Grouping inside those 49 Bucket 4 rows (`49 -> d_B4 = 46` axes, once every law is built):**
   - `RN-071` (PTH1R sites) and `HP-032` (GR sites) cancel from non-depleting receptor occupancy (`-2` if fixed, or retained strictly as zero-effect falsifier controls).
   - `HP-033`, `HP-034`, `HP-035` (GR `k_on`, `k_off`, `eps`) collapse over slow 6 h cortisol dynamics to `[K_D = k_off / k_on, eps]` (`3 -> 2`, `-1`).
   - `RN-070`, `RN-078`, `GI-038` (3 titration/absorption splays) stay as `3` axes (collapsing to `43` if [research 05](../../research/issues/05-parameters.md) routes them to numerical widths).
   - Macula densa pairs `RN-024`/`RN-025` (TGF) and `RN-033`/`RN-034` (renin) remain `4` axes (`3` if `body 17` unifies `x0`).
   - Net **Bucket 4 active dimension: `d_B4 = 46`** (`49 - 2 - 1`; or `44` excluding check-spending `MT-001` and `RN-040` from Stage B response fitting). The first draft wrote `44`, `41` and `42` for these three figures, two fewer than its steps give.
   - What the engine holds now is smaller. The 13 keyed rows are the only axes a run could vary at `9a7469ff`, before any grouping.

3. **Tier-1 Stage A dimension (`d_A = 77` axes) for `research 05` sourcing triage:**
   - Combines `d_B4 = 46` with `31` high-priority grouped Bucket 1 axes:
     - `~9` composite receptor `K_D`/`eps` axes collapsed from the `31` `none named` receptor rows (`V2`, `PTH1R`, `InsR`, `GcgR`, `AT1`, `MR`, `NPR-A`, `M3`),
     - `10` non-receptor `none named` sourced rows (`CV-005`, `CV-008`, `CV-010`, `CV-034`, `CV-061`, `FL-004`, `GI-001`, `MT-021`, `MS-017`, `MS-018`; duplicates `FL-008`/`RN-006` and retired `GI-028` excluded),
     - `~12` grouped non-check-spending `at-the-instant` and wide-uncertainty Bucket 1 pairs (`CV-083`/`084`, `EN-028`/`029`, `GI-002`/`003`, `GI-033`..`035`, `NS-017`/`018` ratio, `NS-020`..`023`, `NS-062`, `HP-007`/`011` net, `MS-008`/`009` turnover, `MT-031`, `NS-037`..`045` grouped).
   - Tight physical-chemistry rows (`BC-001`..`027`) and already-pinned `data/` constants (`CV-036`..`038`) stay fixed in Tier-1 (`100`-axis extended bound covers adding any remaining wide-range Bucket 1 rows if needed).

4. **Sampling transforms:** Positive rates, conductances, and affinities sample log-uniformly; bounded shares (`[0, 1]`) and linear offsets sample uniformly on `[low, high]`; correlated anthropometric/allometric traits follow sourced covariate links in `PopulationRecord` when [research 05](../../research/issues/05-parameters.md) supplies them.

## 5. Response summaries and hold-out target firewalls

Stage A evaluates `M ≈ 25-30` scalar response summaries extracted from a unified 6 h morning protocol (`06:55` admission -> `07:00` sit-up posture ramp -> `07:30` standardized meal, paired with a counterfactual run without the meal per `crates/ng-validate/src/day.rs:236-242`; without that run the meal rows the engine scores read `NoCounterfactual`, `day.rs:324-325`), plus a separate 24 h / sleep stretch only when evaluating sleep/circadian axes (`NS-053`, `NS-047`..`052`):

| Summary group | Engine source / `crates/ng-validate/src/` | Concrete scalar summaries (`y_m`) | Stage B role & hold-out firewall |
|---|---|---|---|
| Posture fast & slow | `day/table.rs:121-182` (`SitUp` rows 1-8) | `HeartRate` rise (`< 15 s`), `ArterialPressure` nadir (`<= 10 s`) & recovery (`30 s`..`end`), `StrokeVolume` fall (`<= 30 s`), `CardiacOutput` rise & late fall, `PlasmaVolume` & `PlasmaAlbumin` shifts (`<= 20 min`), `SodiumExcretion` (`<= 30 min`) | Eligible for Stage A ranking; **hold out** Sarafian and Miles-Chan 2016 tilt-sine data (`W252`, [research 05](../../research/issues/05-parameters.md#L38)) forever from Stage B |
| Meal & metabolic | `day/table.rs:183-219` (`Meal` rows 9-13) and [ticket 14](14-first-experiment-on-the-whole-body.md) | In the engine at `9a7469ff`: `PlasmaOsmolality` & `PlasmaSodium` against counterfactual (the two scored rows), and the unscored vasopressin, sodium, potassium and chloride excretion and plasma K rows. Not in the engine yet: postprandial glucose peak & 2 h/4 h AUC, insulin peak & tail, glucagon, gastric emptying half-time, `VO2`, `VCO2`, `RQ`, plasma lactate, core/skin temp shift. The engine's meal is water and three salts (`crates/ng-validate/src/day.rs:37-42`), no `DayQuantity` holds a fuel, a gas or a temperature (`day.rs:264-280`), and the whole body holds insulin and glucagon secretion and basal glucose at zero (`crates/ng-validate/src/body/packet.rs:497-509`). Those summaries wait for body tickets 13 to 16 | **Hold out** Visentin 2015 (`W017`, [research 05](../../research/issues/05-parameters.md#L40)) and the non-`research 13` meal study ([ticket 14](14-first-experiment-on-the-whole-body.md#L31)) from Stage B; partition [research 08](../../research/issues/08-validation-targets.md) targets under `L8` before any Stage B wave |
| Baseline lens & stability | `healthy.rs`, `certificate.rs` | Baseline `BareLens` resting readings ([research 06](../../research/issues/06-healthy-ranges-and-lens.md)), `Certificate` max 6 h/24 h drift ratios per conserved quantity | Diagnostic stability & anchor-range check; never fit check-spending parameters to their own anchor |

**Explicit firewall on the 19 check-spending constants (`research 16` line 437):** Stage A may rank how strongly any of these 19 constants moves non-target readings, **but Stage B must never include a summary in `I(x)` if fitting that parameter spends that summary's check**:
- `RN-040` (`S_P`) never fits against pressure-natriuresis (`RN-L04`);
- `NS-028`, `NS-031` (`G_c`, `G_p`) never fit against CO2 ventilatory response (`RS-V04`);
- `NS-034` (`W`) never fits against wake-to-NREM PCO2 rise (`NS-A01`, `RS-V02`);
- `NS-047`..`NS-052` never fit against core temperature rhythm (`MT-A03`, `MT-V01`) from the same study;
- `MT-001` never fits `sum(K_t)` against `BC-A06`; `MT-004` never fits against `MT-A01`;
- `RS-003`, `RS-005`, `RS-006`, `RS-013` never fit against `RS-A04`/`RS-V03`/`RS-A02`;
- `EN-040`, `HC-002`, `RN-063` never fit against `EN-A02`, `HC-A01`, `BC-A01`.

## 6. Admission-refusal accounting (`L9` feasibility surface)

Per rule `L9`, **zero refused or faulted runs are discarded**. Every sampled `x_i` writes an immutable manifest entry holding `(spec_id, packet_hash, status, failure_enum, wall_ms)` keyed to exact Rust refusal types:

- **Pre-solve / packet refusals (`AdmissionError`, `crates/ng-contract/src/failure.rs`):** `ParameterOutsideSet`, `OutsidePopulation`, `NoPopulation`, `FitUnread`, `AnchorUnread`, `NonFinite`.
- **Admission solve refusals (`InitializationFailure`, `crates/ng-contract/src/admission.rs:1899-1934`):** `Inconsistent { row }`, `Undefined { law }`, `NetBoundaryFlow { quantity }`, `GuardSweep { point }`, `Unresolved { variable }`, `SecondWrap`, `Unpaced`, `NoRank { row }`, `CrossingRank { variable }`.
- **Stepping refusals & engine faults (`NumericalRefusal`, `EngineFaultKind`, `crates/ng-contract/src/failure.rs:476-508`):** `Accuracy`, `IterationLimit`, `TrialDomain`, `StageResidual`, `LedgerOverflow`, `GuardInside`, and terminal `EngineFaultKind::NoAdmissibleStep`.

A refusal at `admit` skips the `S_run * w` stepping time, but it costs more than an admission and not less. Every whole-body refusal the tickets report took 2.2 to 3.7 times as long as an admission on the same worker: 545, 735 and 758 s against 239 to 247 s, and 2,515 s against 689 to 760 s at an earlier commit (native-contract 82); about 915 s against 372 s (body 11). A solve that ends in a refusal runs to its limits first (inferred). Stage A trains a binary GP feasibility classifier `P_feasible(x)` across all `N` manifest rows (`1` = `admitted_complete`, `0` = `admit_refused | run_faulted`), computes summary GPs conditionally on `admitted_complete` rows (`N_ok`) so failed trajectories never corrupt `y_m`, and reports **two rankings**: conditional summary sensitivity `S_Ti(y_m | feasible = 1)` and feasibility-boundary sensitivity `S_Ti(P_feasible)`.

## 7. Parameterized run-budget table (`2.5x`, `1x`, and `10x` real time)

These figures were computed before [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) reported any whole-body speed. Body 30 is still open, and its figures replace these when it resolves. The table evaluates the parameterized cost equation `Hours = N * (T_admit + S_run * w) / 3600` using `T_admit = 200 s` (a round figure: `body 26`'s quiet runs admitted in `177.5` to `190.6 s`, and `native-contract 86` counts about `240 s`) and `S_run = 21,600 s` (`6 h`) across three reference speeds: `2.5x` (`w = 0.40 s/sim-s`, near [body 26](../../body/issues/26-cost-budget.md)'s quiet baseline on holds), `1x` (`w = 1.00 s/sim-s`, live real-time floor), and `10x` (`w = 0.10 s/sim-s`, [SRS section 6](../../../docs/srs.md#6-fidelity-aims) unbounded aim). The table's hours are for single runs. The meal rows need the control run, and there are two ways to make it. A control that shares the 35-minute prefix and runs to the end of the meal's 4 h window makes the pair `36,000` simulated seconds, `1.65` times a single run at `2.5x` (`1.61` at `10x`). The engine's own day check reruns the control from the same start (`crates/ng-validate/src/day.rs:433-434, 471-473`), which makes the pair `38,100 s`, `1.75` times (`1.70` at `10x`). Sharing the prefix saves `2,100` of those `38,100 s`, `5.5%`, and [ticket 18](18-teacher-library-and-start-proposals.md) section 1.2 keeps fresh runs as the default.

| Parameter regime | Shape B (`N_B = 6d + 20`) | Shape A (`N_A = 6(d+1) + 10 d_act + 20`) | `2.5x` (`w = 0.40`, `2.46 h/run`) Shape B / Shape A | `1x` (`w = 1.00`, `6.06 h/run`) Shape B / Shape A | `10x` (`w = 0.10`, `0.66 h/run`) Shape B / Shape A |
|---|---|---|---|---|---|
| Bucket-4 active (`d = 46, d_act = 15`) | 296 runs | 452 runs | **726.8 h** / 1,109.9 h | **1,792.4 h** / 2,737.1 h | **194.0 h** / 296.3 h |
| Tier-1 Stage A (`d = 77, d_act = 20`) | 482 runs | 688 runs | **1,183.6 h** / 1,689.4 h | **2,918.8 h** / 4,166.2 h | **316.0 h** / 451.0 h |
| Extended screen (`d = 100, d_act = 25`) | 620 runs | 876 runs | **1,522.4 h** / 2,151.1 h | **3,754.4 h** / 5,304.7 h | **406.4 h** / 574.3 h |

If `20%` of draws are refused at `admit`, each at 2.2 to 3.7 times an admission (section 6), Shape B worker-hours drop by `18.3%` to `19.0%` at `2.5x` and by `13.8%` to `16.3%` at `10x`. On a 16-worker cluster, the `d = 46` single-run screen completes in `~1.9` days at `2.5x` or `~12.1` hours at `10x`. With the control run, the `d = 46` design is `1,200` to `1,270` worker-hours at `2.5x`. With only the 13 rows keyed now, before grouping, `N_B` is `98` runs and `240.6` worker-hours at `2.5x`.

**The budget in bytes** (arithmetic from figures the tickets state). The first draft counted none.

- Links: about `276 GB` for a six-hour run until native-contract 95 lands, so `81.7 TB` for `296` runs and `4.4 TB` with 16 runs in flight. The worker that measured the links has `133 GB` free, so one run does not fit on it.
- Capture: about `2.6 GB` a run with every row and column kept at every beat ([ticket 18](18-teacher-library-and-start-proposals.md), section 1.5), so `0.78 TB` for `296` runs.

## 8. Synthetic falsifier tests

Before running Stage A on the whole body, the Stage A Python pipeline must pass four pre-registered synthetic tests on a fast coupled synthetic ODE and a small synthetic compiled body:

1. **Planted zero-effect constant ranks bottom:** Inject a non-interacting constant (mirroring non-depleting receptor site counts `RN-071`/`HP-032` or `MS-001` stir rate on non-chance summaries). Pass bar: normalized total Sobol share `S_Ti <= max(0.005, feature_floor_var)` and bottom rank among all active axes.
2. **Planted dominant constant ranks top:** Inject a known dominant sensitivity driving a target summary `y_1` (`>= 60%` of planted variance). Pass bar: GP-ARD inverse lengthscale and `S_T1` both place it rank 1 for `y_1` with `S_T1 >= 0.50`.
3. **Planted collinear pair flagged unseparated:** Inject a pair `(theta_a, theta_b)` entering only as `theta_a / theta_b` or `theta_a * theta_b` (mirroring `EN-021`/`EN-025` or `HP-007`/`HP-011`). Pass bar: the diagnostic detects ridge collinearity (joint interaction `S_Tab` or aligned directional sensitivity with orthogonal directional `S_Ti <= 0.01`, plus singular GP gradient covariance along `(theta_a, theta_b)`) and emits an `unseparated` flag rather than promoting one member as individually pinned.
4. **Held-out GP interval calibration & `L9` classification:** Evaluate `n_val = 20` held-out feasible runs, and a second held-out set that holds at least 10 admitted and at least 10 refused draws. Three pass bars, each a model default:
   - Empirical `95%` predictive interval coverage is at least `16/20`, with zero systematic bias.
   - The intervals are not vacuous: their mean width is under half the summary's range over the design. A bar on coverage alone cannot fail an interval that is too wide, which is what the first draft's `[0.80, 1.00]` allowed.
   - `L9` balanced accuracy, the mean of the rate right on admitted draws and the rate right on refused draws, is at least `0.80`. The first draft asked `>= 85%` plain accuracy on 10 mixed draws, which a classifier that always answers "feasible" passes `74%` of the time when `90%` of draws admit (arithmetic).

## What is still owed

- Ticket 15's decision on where varied values come from, and a range for each axis from research 05.
- A store that bounds a run's links (native-contract 95).
- The axes and the summaries read again at the commit where Stage A would run: which rows have a crate key by then, and which day quantities exist.
- The budget restated from body 30's speed, with the control run and the bytes counted.

## Comments

- Audited at `origin/main` (`ffcd1a7a`) with a read-only Python parser verifying all `558` rows of [research 16](../../research/issues/16-constants-and-what-binds-them.md) (`409 + 32 + 2 + 115 = 558`), reconciling the `141` slice keys vs `152` live crate keys on `origin/main` (`body 12a`'s `11` `mt.*` keys).
- Verified checks after writing this ticket: `bash tools/verify.d/10-docs.sh`, `bash tools/verify.d/30-data.sh`, `python3 -I tools/frontier` (confirming clean exit `0` and zero `Other:` entries), and an automated hygiene scan for em dashes, dead links, placeholder markers, ISO dates, bare `#N`, and [GLOSSARY](../../../GLOSSARY.md) `_Avoid_` words.

From a review of this ticket, read at `9a7469ff`:

- **The audit holds.** A second parser recounted research 16 and matched every count: 558 rows; 411, 32, 38, 57 and 20 by route; each per-system cell; the four ID lists; the 19 check-spending rows; the 44 sourced rows with no work named. The budget table's arithmetic and all 22 refusal and fault variants in section 6 held too.
- **Open again.** Stage A is not unblocked (section 2). Of its 49 rows, 13 have a crate key and 36 do not. Its runs need varied packets and a range for each axis. One run's links do not fit on the worker that measured them.
- **Section 4.** The stated steps give 46 axes and not 44, so 296 runs and not 284, and 77 where the first draft said 75. Derived rows do not move by themselves.
- **Section 5.** Most of the meal summaries are not in the engine's day yet. They are what body tickets 13 to 16 build.
- **Sections 6 and 7.** A refusal costs 2.2 to 3.7 times an admission in every case the tickets measured. The hours now say what the control run adds, and the bytes are counted.
- **Section 8.** Test 4 could not fail on an interval that is too wide, and its classifier bar passed a constant answer most of the time. Both have a bar that can fail now.
- **Not checked.** The physics behind each grouping, the two approximate group counts behind the 31 Tier-1 axes, and the cited works.
