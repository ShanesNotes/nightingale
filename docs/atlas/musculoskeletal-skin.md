# MS. Musculoskeletal and skin

Muscle, neuromuscular junction, bone and skin. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

The neuromuscular junction and muscle force, then skin as a perfused barrier with a pressure-injury dose model.

## What it owns

Junction transmission, muscle force and fatigue, the respiratory pump force, lean mass, bone exchange, skin perfusion and pressure injury dose, fracture and compartment pressure.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S1 (fracture bleeding edges, compartment pressure), S4 (K, phosphate, CK, myoglobin release; bone exchange), S5 (glycogen, amino acids), S6 (shivering, activity), S9 (creatinine production; receptor density).
- **Reads:** S2, S7, S8, S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** RS (respiratory pump force and fatigue), CV, RS, NS and GI (position vector and gravity, interface pressures to skin).
- **In:** NS (motor drive, shiver drive), RS (pump load), FL (skin and compartment interstitial volume), MT (temperature).

## Place in the contract ownership register

Work and ATP: MS charges muscle work (A section 3). Solutes: MS owns its tissue reactions.

## Injury, failure and demise

Deconditioning, residual blockade, myasthenic weakness, rhabdomyolysis, compartment syndrome, hyperthermic syndromes, fracture. **Demise:** respiratory pump failure (through RS) and hyperkalemia from lysis (through BC and CV).

Decision [0004](../decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Junction in milliseconds, blocker onset in 1 to 2 min, fatigue in minutes, pressure injury over hours, CK peak over days.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | None as a module |
| Pulse reference | No matching Pulse system model; parts appear as Pulse actions or inside other models |
| Proposed crate | `ng-musculoskeletal` (see the [crate map](crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](../vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 11 (see the [build order](build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

Respiratory pump force is MS state that RS reads, not an RS parameter.

## Open

Pressure-injury thresholds in humans (arena atlas flag). Bone as a reservoir only.

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card MS at line 1685; injury table at line 1761; timescales at line 1776. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), musculoskeletal section at line 725.
- Contract pass 1: [synthesis](../council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](../council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](../council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
