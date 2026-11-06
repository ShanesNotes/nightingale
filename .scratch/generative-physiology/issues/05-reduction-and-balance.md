# 05: Named totals, derived transfers and their balance on teacher data

Type: task
Status: open
Blocked by: 03, 04
Runs on: written on the workstation; compiled, tested and checked against the audit's runs on the heavy-run machine

**What it builds:** the reduction that turns a teacher run into what the named model learns from. A schema file lists the named totals. The driver compiles the schema against the arena body once, derives the transfers from the body's columns, proves the grouping with an exact integer identity, and writes one compiled reduction that the Python side reads and never re-derives. The study then checks, on every beat of every audit run, that each named total's change equals its transfers' net extents. The same machinery compiles R's richer reduction from a one-line schema.

## Usage

```text
gp-driver catalogue --schema tools/generative-physiology/spec/named-v1.txt --out DATA/reduction-named.json
gp-driver catalogue --schema tools/generative-physiology/spec/richer-v1.txt --out DATA/reduction-richer.json
python3 -I tools/generative-physiology/gp.py audit --data DATA --source-commit "$C" --report   # now with balance rows
```

```python
reduction = Reduction.read(data / "reduction-named.json")   # refuses a file whose digest line does not match it
view = beats_view(RunData.open(run_dir), reduction)          # totals at edges, net extent per transfer per beat
report = balance(RunData.open(run_dir), reduction)            # both identities on every beat, with residuals
```

The schema, one line per named total, in the words the code uses for sites and spaces (`crates/ng-validate/src/slice.rs:14-50, 202-254`):

```text
gp-schema 1
mode listed
total "arterial blood" water: "systemic arteries" Lumen
total "central blood" water: "right atrium" Lumen, "right ventricle" Lumen, "left atrium" Lumen, "left ventricle" Lumen, "pulmonary artery" Lumen, "pulmonary vein" Lumen, "central veins" Lumen, "pulmonary bed" Plasma
total "splanchnic blood" water: "splanchnic bed" Plasma, "splanchnic veins" Lumen
total "leg blood" water: "leg bed" Plasma, "leg veins" Lumen
total "renal blood" water: "glomerular bed" Plasma, "peritubular bed" Plasma
total "leg interstitium" water: "leg bed" Interstitium
total "splanchnic interstitium" water: "splanchnic bed" Interstitium
total "other interstitium" water: "pulmonary bed" Interstitium, "glomerular bed" Interstitium, "peritubular bed" Interstitium
total "tubule and duct" water: "renal tubule" Lumen, "collecting duct" Lumen
total "bladder" water: "bladder" Lumen
total "norepinephrine stored" norepinephrine: "sinoatrial junction" Vesicle, "myocardial junction" Vesicle, "arteriolar junction" Vesicle, "venous junction" Vesicle
total "norepinephrine released" norepinephrine: the same four junctions, Cleft and Bound
total "renal norepinephrine stored" norepinephrine: "renal junction" Vesicle
total "renal norepinephrine released" norepinephrine: "renal junction" Cleft, Bound
total "acetylcholine stored" acetylcholine: "sinoatrial junction" Vesicle
total "acetylcholine released" acetylcholine: "sinoatrial junction" Cleft, Bound
held water: Cells at every capillary bed, Fluid at every enclosed space
```

R's schema is `gp-schema 1`, `mode each-row`, `memories all`. Every amount row of every species is its own total, and the nine memory rows are carried (five valve flows, the rhythm's phase and activation, the afferent filter and the baroreflex set point, as `crates/ng-validate/tests/arena_body.rs:128, 164, 369-370, 683, 757` names them).

**Why these sixteen.** Pools that relax within a beat are merged, and pools whose walls, heights or rates differ stay apart. Capillary plasma joins its bed's veins, and cleft joins bound, because binding settles in about 0.1 s while reuptake takes 4 s (`crates/ng-validate/src/slice.rs:857, 864-865`). Leg and splanchnic veins stay apart, because their walls differ (0.4 against 0.1, unstressed volume 566 against 1,064) and they sit on opposite sides of the hip (`slice.rs:30-31, 739-740`). The renal junction stays apart because its `rc.ne-off` is 40 where the other junctions' is 10 (`slice.rs:865-867`). Each junction is closed, because release and reuptake are its only transmitter transfers (`crates/ng-nervous/src/lib.rs:13-46`), so stored and released totals close where a bound-only total would not. Sodium, potassium, chloride, albumin, vasopressin and energy ride the same columns but stay outside the named model's balance claim.

## Types and signatures

```rust
// driver/src/reduction.rs
pub struct Schema { mode: SchemaMode, totals: Vec<TotalSpec>, held: Vec<PoolPattern>, memories: MemoryChoice }
pub enum SchemaMode { Listed, EachRow }
pub struct TotalSpec { pub name: String, pub species: Species, pub pools: Vec<PoolPattern> }
pub struct PoolPattern { pub site: SitePattern, pub space: Space }   // a site key, or every site of one class
pub enum MemoryChoice { None, All }
impl Schema { pub fn parse(text: &str) -> Result<Self, SchemaError> { todo!() } }

pub enum End { Total(usize), Receipt(&'static str) }
/// One derived transfer: every column of a carried species with the same two ends and the same origin.
pub struct Transfer { pub name: String, pub origin: LawName, pub from: End, pub to: End,
                      pub columns: Vec<(ColumnId, Orientation)> }
pub enum Orientation { Along, Against }   // Against swaps a column's forward and reverse parts
pub struct Reduction { totals: Vec<Vec<RowId>>, held: Vec<RowId>, memories: Vec<RowId>,
                       transfers: Vec<Transfer>, digest: Digest }
impl Reduction {
    /// Refuses a pool claimed twice, a pattern that matches no row, a row of a carried species that is in
    /// neither a total nor the held group, a held row that a column moves, a carried column with an end it
    /// cannot classify, and a column for which P S = B A fails in integers. Drops a group whose two ends are
    /// one total.
    pub fn compile(body: &CompiledBody, catalogue: &Catalogue, schema: &Schema) -> Result<Self, ReductionError> { todo!() }
    pub fn write(&self, out: &Path) -> io::Result<Digest> { todo!("reduction.json, with its digest") }
}
```

Here P maps amount rows to totals, S is the body's column incidence (`Column::terms`, `crates/ng-contract/src/body.rs:313`), A maps columns to transfers with their orientation and B is the incidence of transfers on totals. The identity says that grouping columns into transfers moves exactly what the columns move.

```python
# study.py
@dataclass(frozen=True)
class Reduction:
    digest: str
    totals: tuple[str, ...]
    transfers: tuple[TransferInfo, ...]       # name, origin, from and to (a total index or a receipt name)
    memories: tuple[str, ...]
    incidence: tuple[tuple[int, ...], ...]    # B, totals by transfers, entries -1, 0 or 1

    @staticmethod
    def read(path: Path) -> "Reduction": ...

@dataclass(frozen=True)
class BeatsView:
    edges_ns: Sequence[int]                   # the beat edges after the run's start
    totals: Sequence[Sequence[float]]         # at each edge
    net: Sequence[Sequence[float]]            # per beat and transfer, from cumulative extents differenced exactly
    readings: Sequence[BeatReading]           # the driver's, never recomputed
    drive: Sequence[Sequence[DrivePiece]]     # per beat, the posture pieces: start, trunk sine, seated, standing

def beats_view(run: "RunData", reduction: Reduction) -> BeatsView: ...
def balance(run: "RunData", reduction: Reduction) -> "BalanceReport": ...
```

A drive piece carries the public posture payload, the trunk's sine and two leg indicators, never the engine's private leg share (`crates/ng-contract/src/boundary.rs:68-75`). Differences of cumulative values are summed exactly with `math.fsum` over both limbs before one rounding.

## Acceptance

- [ ] `gp-driver catalogue` on the named schema prints the derived transfers with their origins; about 26 are expected (the stroke, three arterial inflows and three returns, four Starling exchanges, three lymph returns, filtration, reclaim, urine flow, urine, gut water, skin water, and release and reuptake for each transmitter pair). The Answer lists the exact set.
- [ ] Planted schemas are refused: a pool claimed twice, a pattern that matches nothing, a water pool dropped from every total and from the held group, and a planted column end that cannot be classified.
- [ ] On every beat of every audit run, each named total's change equals the sum of the column terms on its pools and the sum of its transfers' net extents, each within 64 times 2^-52 of the sum of the magnitudes involved. Each transmitter pair's sum also equals, at every edge, the sum of `HistorySegment::inventory` over the conserved combinations whose terms are its pools (`crates/ng-record/src/history.rs:565-567, 613-615`). Release, reuptake and binding join pools only at one junction (`crates/ng-nervous/src/lib.rs:39, 46`, `crates/ng-receptor/src/lib.rs:85`), so each junction is its own combination (`crates/ng-contract/src/conservation.rs:37-38, 113`): four for the pooled norepinephrine pair, one for the renal pair and one for acetylcholine.
- [ ] R's schema compiles, and its identity holds on the same beats.
- [ ] The audit report gains these balance rows and each transfer's reversal share, its gross reverse part over its gross forward part per beat.

## Non-goals

- Deciding whether the sixteen totals suffice (ticket 07), and the models (ticket 08).
- Retesting the engine's own conservation, which its certificate owns. The certificate is no short-run check either, because its coverage type needs more than 24 hours (`crates/ng-validate/src/certificate.rs:43-53`).
