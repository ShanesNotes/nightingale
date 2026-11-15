# 61: The vein store's tests hold more of it

Spec: `crates/ng-validate/tests/veins.rs`, `crates/ng-cardiovascular/src/walls.rs` and [native-contract 57](57-the-store-below-a-veins-unstressed-volume.md)

Type: task

Status: open

Blocked by: none

**Why.** The first milestone (SRS invariant I19) is judged on the walls' energy ledger, and [native-contract 57](57-the-store-below-a-veins-unstressed-volume.md) made a marked vein's store below its unstressed volume `V_u` the work of its pressure. Two tests hold it against a sum of the vein's own pressure. The check of nightingale#168 ran 21 mutants of its own through them: 14 were caught on a value, and six survived (`~/lanes/logs/ng-wave-4/nc57/verify-coordinator/VERIFY-pr168.md`, items 2 and 3).

**What survives today.**

- A factor on the store of 1.00005, 0.9999 or 0.9995. The store sits at 0.82 to 0.86 of the band, so it may rise by 0.006% to 0.025% and fall by 0.06% to 0.31%.
- The store half as large again at the splanchnic veins alone. The tests stage the central veins and the leg veins, two of the four marked veins.
- The width that smooths the volume the store is read at, doubled or cut to a twentieth, and the fraction read at `V` in place of the smoothed volume. Each changes the store by less than the soft floor's offset.
- The term `vein collapse stroke work`, dropped or doubled. Its value is zero at every marked vein of the slice, because none has an active elastance. Only the model hash and the seals show the change.

- [ ] The tests hold the store's growth less the work to the soft floor's offset, `E (1 - share) w^2 asinh((V - V_u) / w) / 4`, with a small tolerance on each side, and a factor of 1.00005 fails
- [ ] The tests stage all four marked veins
- [ ] A test holds which width smooths the read, or the Answer says why none is needed
- [ ] A fixture with a marked vein that contracts holds the stroke-work term's value: an active elastance above zero, staged below `V_u` in systole. The slice binds each site's active elastance once, so it needs a fixture of its own, a small body in the cardiovascular crate's tests or a slice case with one more vein
- [ ] The law's doc says the wall holds the work of its pressure up to the soft floor, as the pressure test's doc does
- [ ] Each new check has a mutant row that is caught on a value

**Non-goals:** the store's form, and the collapse branch's pressure.
