# 100: The admitted start is every pool's rest

From: [body ticket 17](../../body/issues/17-volume-hormones.md)'s plan, its sections "The diagnosis" and "The cause upstream"

Type: task

Status: open

Blocked by: 85

Blocks: [body ticket 17](../../body/issues/17-volume-hormones.md)

**Why.** After admission the body leaves salt balance, and the drift starts at the lung bed, not in the kidney. The plan for body ticket 17 traced it at main `89fa470`. Every value is synthetic, and each rate is a mean over the span it names. The admitted start gives the pulmonary bed's interstitial water a lymph flow about equal to its Starling flow, while the airway's lining also draws water from that pool. So the pool drains. Its three routes, in water a second:

| Span | Starling in | Lymph out | Lining's water out | Sum | The pool's own rate |
|---|---|---|---|---|---|
| 20 to 320 s | 0.009949 | 0.009704 | 0.00421 | -0.00397 | -0.00399 |
| 2,548 to 2,846 s | 0.010445 | 0.006723 | 0.00424 | -0.00052 | -0.00051 |

- The lining draws `V'_A (σ - i) / v_w` on a packet ventilation (`crates/ng-respiratory/src/airway.rs`), constant by construction, so the breath cannot move it. Cell water's route nets zero here.
- The pool drains until its pressure cuts the lymph to the Starling flow less 4.2e-3. Against the pool's pressure the lymph's slope is 0.0147, Starling's -0.0024 and the pool's compliance 20, so it settles with a time constant of 20 / 0.0171 = 1,170 s. Awake it falls from 428.49 to 424.42 by 2,900 s (-0.95%) and settles near 423.8 by about 6,000 s.
- The water it sends to the blood holds 0.246 albumin a unit of water against plasma's 0.428, so the plasma's albumin dilutes: glomerular albumin falls 0.046% from 170 to 2,750 s. The oncotic term raises filtration by 0.047%, and the bed's pressure and the lumen's do not account for it. The kidney's duct reclaims near its capacity and passes the rise to the urine about 23 times over: the duct's sodium outflow is 1.00038, 1.0113 and 1.01541 a second over 20 to 320, 1,199 to 1,500 and 2,548 to 2,846 s (+1.50%). Chloride and potassium keep sodium's factor.
- The breath is not the cause. A run with both chemoreflex gains at 0 and the drive scale held at exactly 1 drifts as main does, from 20 to 320 s against 1,199 to 1,500 s: lung water 428.514 to 425.689 against main's 428.492 to 425.611, and the duct's sodium outflow 1.00028 to 1.01088 against 1.00038 to 1.0113.

**What is not known: why admission leaves the pool off its rest.** The pool's rate after admission is about 9e-6 of itself a second. Admission's Newton stops on an update under 8 epsilon of each unknown or on a stall under 1e-11 (`crates/ng-validate/src/initialize.rs`, the constants `NEWTON_STOP` and `NOISE`), so a loose stop is unlikely. _Inferred_ and untested: the pool's rest as admission solves it leaves the lining's water out, or the lymph was fitted before the lining's water was built ([body ticket 10](../../body/issues/10-lungs-breathe.md)'s fourth PR).

**Why after 85.** Native-contract 85's second PR moves the body's pins first, and this ticket's fix moves them again.

- [ ] The cause is traced to the row or the packet value that leaves the pool off its rest, and the PR says which
- [ ] The fix is at that owner. A suite test on the awake run bounds the pool's rate over whole breaths, and one run outside the suite shows it: the duct's sodium outflow moves under 0.1% over 1,500 s from admission
- [ ] An audit, run once and written into this ticket: every pool whose mean rate over whole breaths of the suite's awake run, carried to a day, passes 0.5% of the pool, with the ticket that owns each. 0.5% a day is a synthetic model default until a reviewed set states the within-subject variation ([research 07](../../research/issues/07-within-subject-variation.md)). Whether the audit becomes a gate is ruled from its length
- [ ] The certificate's bounds are sized again by `RULE` (`crates/ng-validate/tests/body_run.rs`) on the suite's run, and the body's trace is updated

**Non-goals:** body ticket 17's loops, and the drift of energy, oxygen and carbon beyond listing them in the audit ([body tickets 12](../../body/issues/12-heat-balance.md) to 16).
