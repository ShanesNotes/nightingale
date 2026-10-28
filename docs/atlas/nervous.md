# NS. Nervous

Central and autonomic nervous system. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

Autonomic outflow is a release rate of norepinephrine or acetylcholine into a target's interstitium, not a dimensionless scale, so a reflex and a drug compete for the same receptor.

## What it owns

Afferent integration, baroreflex and chemoreflex, autonomic release (sole writer of S10), respiratory drive, thermostat set point, a labeled consciousness index, intracranial pressure and volume, brain injury integrals.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S7 (hypothalamic releasing signals), S10 (sole writer).
- **Reads:** S1 (afferents), S2, S3, S4 (pH, Na, osmolality), S5 (brain glucose), S6 (thermoreceptors), S8.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** RS (drive and cough), MT and MS (thermostat, shiver and sweat drive, motor drive), EN (CRH, vasopressin and oxytocin drive, circadian phase).
- **In:** CV (baroreceptor afferents), RS (lung afferents), HP (ammonia), IM (PGE2 set point shift), FL (brain water), MT (temperature, O2 debt).

## Place in the contract ownership register

Autonomic transmitters: NS owns release transfers and reuptake and clearance reactions (A section 3).

## Injury, failure and demise

Sedation and encephalopathy, delirium, global hypoxic-ischemic injury, stroke, raised ICP and herniation, seizure, spinal cord injury. **Demise:** the time integral of cerebral O2 deficit above threshold turns reversible depression into permanent loss; herniation is a staged failure. This is the anoxic brain injury that decision 0004 requires.

Decision [0004](../decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Milliseconds (conduction) to seconds (vagal and sympathetic responses), minutes (central chemoreflex, consciousness), and resetting over days.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | `vitals-nervous` (`baroreceptor`, `chemoreceptor`, `brain`); brain injury also in `vitals-cardiovascular` (`brain_injury`) |
| Pulse reference | `NervousModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-nervous` (see the [crate map](crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](../vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 5 (receptor layer and reflexes), 6 (hypoxic-ischemic injury for demise) (see the [build order](build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

NS is the only writer of autonomic release. Reflex outputs act only through receptors.

## Open

Consciousness index shape (arena atlas flag: no identified model exists; keep it labeled). Transmitter release and clearance rates (fitted).

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card NS at line 367; injury table at line 460; timescales at line 473. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), nervous section at line 179.
- Contract pass 1: [synthesis](../council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](../council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](../council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
