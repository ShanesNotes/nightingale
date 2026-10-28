# <ID>. <System name>

<Scope in one line>. Status: **proposed card** | **accepted card**. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

The one behavior the system must get right before anything else, in one or two sentences.

## What it owns

The laws and compartments it owns. A system owns laws, never a shared pool.

## Shared substrates

- **Posts flux to:** substrate ids from the [coupling map](coupling-map.md#shared-substrates), each with the edge or process.
- **Reads:** substrate ids it reads.

## Direct couplings

- **Out:** system and signal, for each wire that bypasses a substrate.
- **In:** the same, inbound.

## Place in the contract ownership register

The rows of the core contract's ownership table where this system owns a process, a reaction or a derived block.

## Injury, failure and demise

Its failure stages, and its role in dying: rhythm modes, injury integrals, or what it passes to the cards that own them. A numerical failure is an engine fault, never a death ([pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md)).

## Timescales

Fastest law to slowest adaptation, in one line.

## Where it lives

| | |
|---|---|
| Crate today | The `vitals-*` crate and module, or "None" |
| Pulse reference | The Pulse REL_4_3_2 model file, or "No matching Pulse system model" |
| Proposed crate | From the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md) |
| Lane | From [engineering-lanes.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/vision/engineering-lanes.md) |
| Build step | From the [build order](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/build-order.md) |
| Proof status | Not started, bench passing, loop passing, or validated, with the test that shows it |

## Decided

What is settled, with the decision or ADR that settled it where there is one.

## Open

What is not settled, and which decision or packet will settle it.

## Sources

Atlas sections with line numbers and document hashes, council outputs, and decision records.
