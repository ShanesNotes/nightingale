# HP. Hepatic

Liver and biliary tree. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

The glucose and lactate hub under insulin and glucagon, then well-stirred drug clearance, so first-pass metabolism, flow-limited clearance and cirrhosis dosing come from one equation.

## What it owns

Hepatic glucose, lactate, ketone and fatty acid reactions, drug metabolism reactions, synthesis of albumin, clotting factors and angiotensinogen, ammonia clearance, sinusoidal resistance and the portal shunt edge, bile.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S1 (sinusoidal resistance, shunt edge), S4 (lactate, urea, albumin charge), S5 (glucose, lactate, ketones, FFA), S6 (heat production), S7 (angiotensinogen, IGF-1, binding proteins, clearance), S9 (factors, fibrinogen, thrombopoietin).
- **Reads:** S2, S3, S8, S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** CV (portal pressure through sinusoidal resistance and shunt), NS (ammonia).
- **In:** GI (portal delivery of absorbed species and bacterial products), EN (insulin and glucagon, portal gradient), IM (inflammatory injury rate), MT (temperature, O2 debt).

## Place in the contract ownership register

Solutes: HP owns its tissue reactions. Hormones and drugs: HP owns its clearance and metabolic reactions. Formed elements and factors: HP owns factor synthesis (A section 3).

## Injury, failure and demise

Shock liver, congestive hepatopathy, drug-induced injury, cholestasis, hepatitis, cirrhosis, acute liver failure. **Demise:** acute liver failure progresses to coagulopathy, hypoglycemia and cerebral edema (NS); the atlas lists transplant or death as its end states.

Decision [0004](../decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Glycogenolysis in minutes, insulin-driven storage within an hour, glycogen exhaustion at 12 to 24 h, INR and transaminase courses over days, albumin and regeneration over weeks.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | `vitals-substances` (`hepatic`), a port of Pulse's `HepaticModel`, which is empty: no state, writes nothing |
| Pulse reference | `HepaticModel.cpp` in Pulse REL_4_3_2 (empty); today's crate cites it line by line in its source headers |
| Proposed crate | `ng-hepatic` (see the [crate map](crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](../vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 9 (metabolic hub) (see the [build order](build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

Hepatic function is built new; there is nothing to port.

## Open

Zonation, shunt fraction by stage, regeneration rate (arena atlas flags). Portal to peripheral insulin gradient (shared with EN).

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card HP at line 654; injury table at line 736; timescales at line 748. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), hepatic section at line 289.
- Contract pass 1: [synthesis](../council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](../council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](../council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
