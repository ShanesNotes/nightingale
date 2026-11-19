# 85: The certificate's windows end on breath edges

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 5, and `crates/ng-validate/src/certificate.rs` (`SlidingWindows`)

Type: task

Status: open

Blocked by: none

Blocks: [body ticket 11](../../body/issues/11-breathing-is-controlled.md)

**Why.** The first milestone (SRS invariant I19) holds the whole body for 48 hours under its certificate, and the certificate is what refuses a slow leak. Once the body breathes, a ten-second window that does not hold whole breaths can catch most of a breath's swing in the gas the lungs hold, so [body ticket 10](../../body/issues/10-lungs-breathe.md)'s second PR set the bounds of the breath's books by that swing, to hold at any breathing rate. A bound that size lets through a steady leak far larger than the steady figures would.

**What the check of nightingale#198 found.** `SlidingWindows` takes only windows of one fixed length (`certificate.rs:78`), so no window can start and end on breath edges. The slowest steady leak each breathing bound refuses today, against the one a bound from the steady figure would refuse, per ten-second window:

- oxygen, 900 a second against 20 a second;
- nitrogen gas, 2,000 against 20;
- carbon, 50 against 9;
- vasopressin, 4e-3 against 3e-4. No other check sees vasopressin.

Fick's check over nine beats takes the blood's store change as slack (oxygen 0.88% of the rate, carbon dioxide 7.6%), because nine beats are not whole breaths.

- [ ] The certificate's windows can end on the run's breath edges, so each breathing book keeps a bound from its steady figure at any breathing rate
- [ ] A steady leak of each old size is refused at any breathing rate: oxygen 20 a second, nitrogen gas 20, carbon 9, vasopressin 3e-4, each shown at two rates, one that divides the window and one that does not
- [ ] Fick's window runs over whole breaths, with the store change as a signed term in place of slack
- [ ] When windows end on breath edges, the hydrogen bound at both paying beds, the coronary bed and the muscle bed, 0.2 each today, is restated. The muscle bed's is twice its worst ten-second change, 8.79e-2, of which the breath's ripple inside one breath is 6.59e-2, so the bound passes a steady leak up to 1.1e-2 a second there
- [ ] It restates the breath's books after [body ticket 11](../../body/issues/11-breathing-is-controlled.md)'s second PR, 11b, with the loop moving the rate, and its two-rate leak tests set the rate by the packet with the drive scale held

**Non-goals:** the breath's rate and its control (body ticket 11), and any bound outside the breath's books.
