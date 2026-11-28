# 74: A reach with a missing pool drops unseen

Spec: `crates/ng-contract/src/compile.rs` (`Reaches::of`)

Type: task

Status: resolved

Blocked by: none

**Why.** Compile keeps a reach only where every pool end is held, so a route whose pool the schema lacks drops out of the compiled body with no word. [Native-contract 70](70-a-beds-heat-pays-the-work.md)'s plan met it: a work route moved to a bed's thermal pool that the schema did not hold would have vanished, and the work with it. That ticket closes both ends of its own two families, stroke and wall work, once a payer is named (`PayerHoldsNoHeat`, `PaidWorkHasNoStore`). Every other route family keeps the pattern, and a schema that loses a pool loses its routes the same way.

- [x] A list, from the compiled slice and the compiled whole body, of each reach compile drops today and why, so the ones dropped by design are known
- [x] Compile refuses a reach dropped for a missing pool unless its family says the pool may be absent, with a test for each family
- [x] The slice's and the whole body's pins do not move

## Answer

### Reaches dropped today (measured from the compiled slice and compiled whole body)

Every reach `Reaches::of` drops on the slice (`16` reaches across `4` families) and on the whole body (`29` reaches across `5` families) is dropped by design because the schema's pools select which sites participate in those families:

| Body | Owner / Family | Count | Sites dropped | Missing pool(s) | Why dropped by design |
|---|---|---:|---|---|---|
| Slice | `NS` / `Release(Acetylcholine)` | 4 | `arteriolar`, `myocardial`, `renal`, `venous` junctions | `Vesicle`, `Cleft` `Acetylcholine` | Only `sinoatrial junction` holds acetylcholine (`NervousPorts::release`). |
| Slice | `NS` / `Reuptake(Acetylcholine)` | 4 | `arteriolar`, `myocardial`, `renal`, `venous` junctions | `Cleft`, `Vesicle` `Acetylcholine` | Same cholinergic junction selection (`NervousPorts::reuptake`). |
| Slice | `Receptor` / `Binding(Junction, Acetylcholine)` | 4 | `arteriolar`, `myocardial`, `renal`, `venous` junctions | `Cleft`, `Bound` `Acetylcholine` | Schema `Bound` pools select where a ligand binds (`ReceptorPorts::binding`). |
| Slice | `Receptor` / `Binding(Blood, Vasopressin)` | 4 | `glomerular`, `leg`, `pulmonary`, `splanchnic` beds | `Bound` `Vasopressin` | Only `peritubular bed` holds bound vasopressin (`ReceptorPorts::binding`). |
| Whole body | `MT` / `Heat(Conduction)` | 4 | `aortic arch`, `carotid sinus`, `heart`, `kidney` | `Thermal` `Energy` | Non-core `TissueSite`s without a heat store do not conduct (`MetabolismPorts::conduction`). |
| Whole body | `NS` / `Release(Acetylcholine)` | 5 | `arteriolar`, `myocardial`, `renal`, `skin`, `venous` junctions | `Vesicle`, `Cleft` `Acetylcholine` | Only `sinoatrial` and `respiratory pump` junctions hold acetylcholine. |
| Whole body | `NS` / `Reuptake(Acetylcholine)` | 5 | `arteriolar`, `myocardial`, `renal`, `skin`, `venous` junctions | `Cleft`, `Vesicle` `Acetylcholine` | Same cholinergic junction selection. |
| Whole body | `Receptor` / `Binding(Junction, Acetylcholine)` | 5 | `arteriolar`, `myocardial`, `renal`, `skin`, `venous` junctions | `Cleft`, `Bound` `Acetylcholine` | Same cholinergic junction selection. |
| Whole body | `Receptor` / `Binding(Blood, Vasopressin)` | 10 | All beds except `peritubular bed` | `Bound` `Vasopressin` | Only `peritubular bed` holds bound vasopressin. |

Two further families outside the active slice/whole-body set also use pool placement by contract:
- `Family::Across(Wall::Cell, Sites::Class(Class::CapillaryBed), Species::Iron)` when owned by `Hematology` (`HematologyPorts::iron_into_cells`), per `docs/architecture/solver-contract.md` and `hematology_crosses_iron_into_cells_only_at_the_beds_whose_cells_hold_iron` in `crates/ng-contract/tests/scopes.rs`. `FluidsPorts::cell_membrane::<CapillaryBed, Iron>` is not exempt and refuses missing iron pools.
- `Family::Heat(HeatRoute::CarriedByBlood)` (`CardiovascularPorts::heat_carried`), where beds without a peripheral tissue heat store strictly below the root have no carriage route; once a peripheral store selects a route, a missing core thermal store is refused with `RouteEndpoint` (as is `MT/conduction` when a peripheral tissue store exists without the core store).

### Enforcement and tests

- Added `CompileDefect::RouteEndpoint { law, site, space, species }` (`crates/ng-contract/src/failure.rs`) and an exhaustive `Family::missing_pool(owner) -> MissingPool` (`crates/ng-contract/src/law.rs`) with no wildcard over `Family`, `Wall`, `Passage`, or `HeatRoute`:
  - `MissingPool::DropEither`: `Release`, `Reuptake`, and `Across(Wall::Cell, CapillaryBed, Iron)` when `owner == Hematology::CARD`.
  - `MissingPool::SelectBy(End::To)`: `Binding` (skips sites without `Bound`, refuses a missing free `Cleft`/`Plasma` pool when `Bound` is present), `Heat(Conduction)` and `Heat(CarriedByBlood)` (skips sites without a peripheral thermal store, refuses a missing core thermal store — both `site: Some(root)` and `site: None` on a non-tissue root — once a peripheral store selects a route).
  - `MissingPool::Refuse`: every other family (`Circulation`, `Valves`, `Sinusoids`, `Starling`, `Lymph`, `Filtration`, `Tubular`, `Reclaim`, `Exchange`, `StrokeWork`, `GravityWork`, `WallWork`, `Dissipation`, every `Across` except `HM` cell-membrane iron, every `Along` passage, `Heat(FromBeds)`, and `Heat(Pumps)`).
- `Reaches::of` (`crates/ng-contract/src/compile.rs`) sorts requested `(LawName, Family)` pairs in canonical key order before validating reaches, so `RouteEndpoint` is invariant to port declaration order, while `check_payers` still runs ahead of `Reaches::of`, preserving `PayerHoldsNoHeat` and `PaidWorkHasNoStore`.
- Tests in `crates/ng-contract/tests/routes.rs` (`every_required_family_refuses_a_missing_endpoint_pool`, `body_energy_families_refuse_a_map_without_a_tissue_root`, `families_that_allow_absent_pools_drop_only_missing_sites`, `selector_families_refuse_a_missing_partner_when_selected`, `fluids_cell_membrane_iron_refuses_a_missing_endpoint_pool`, `route_endpoint_refusal_is_invariant_to_port_declaration_order`) and `crates/ng-contract/src/compile.rs` (`fluid_families_forbid_absent_pools`) cover every family, backed by `tools/gate/mutants.d/nc74-reaches.toml` (`reach-missing-pool-dropped`, `reach-missing-root-store-dropped`, `reach-selector-drops-missing-partner`, `reach-iron-exemption-not-scoped-to-hematology`, `reach-endpoint-declaration-order`).
- Updated `sweat::a_placed_carriage_tissue_read_requires_the_core_store` (`crates/ng-contract/src/sweat.rs`) to request `conduction` and `heat_carried` only when `with_core` is true, keeping §7's `NoSuchRead` test and `carriage-tissue-end-without-core-store` mutant intact.
- Slice (`ng-validate --test trace`) and whole-body (`ng-validate --test body`) pins are unchanged.
