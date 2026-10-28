// Ledger rounding experiment for solver-contract review r1 (the integer-ledger question).
//
// Synthetic magnitudes only: inventories are S and S*1e-6 for a unit scale S, and every transfer is
// a stated ratio of the pool it leaves. No physiological value appears.
//
// The body is four pools. Pools 0, 1 and 2 (about S each) pass a high-turnover flow around a ring,
// rho*S per 2 ms step with a one-second pulsation and a different phase and jitter per column.
// Pool 3 (S*1e-6) exchanges a cross-scale flow with pool 0 both ways. An exterior intake enters
// pool 0 on a daily schedule and an exterior output leaves pool 2 at the same daily mean, both
// kappa*S per step with kappa = rho*1e-6. Every pool returns near its start each beat and each
// day, so the closure defect is rounding alone.
//
// Each accepted step applies the contract's update x_next = x_base + B*J: one extent per column,
// debited and credited with the same number, and adds the exterior extents to the history's
// cumulative inflow and outflow. The closure defect over the run is
//     D = (sum x(T) - sum x(0)) - (In - Out),
// evaluated in double-double from the stored values, so D is the error the stored ledger itself
// carries. Exact arithmetic gives D = 0.
//
// Modes: f64 (pools and accumulators plain f64); dd (pools and accumulators double-double through
// Knuth's two-sum); i64 (every extent rounded independently to an integer number of quanta
// q = S * 2^-56); i64c (the same quanta with a fractional carry per column, as review R3 proposed:
// each column keeps its sub-quantum remainder and adds it to the next extent). Both integer ledgers
// close exactly by construction, so their cost is reported as drift: the largest distance of any
// pool from the double-double pool at the end of the run, over the exterior exchange.
//
// Cores: "2ms" applies 86_400_000 accepted steps (48 h at a 2 ms step, a core whose global step
// follows the valves). "beat" applies 172_800 macro steps of 500 bins (48 h at one macro step per
// simulated second); its extents are f64 sums of the bins, applied once per macro step.
//
// Build and run: rustc -O --edition=2024 ledger-rounding.rs -o ledger-rounding && ./ledger-rounding

#[derive(Clone, Copy, Default)]
struct Dd {
    hi: f64,
    lo: f64,
}

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

impl Dd {
    fn of(x: f64) -> Self {
        Dd { hi: x, lo: 0.0 }
    }
    fn add(&mut self, x: f64) {
        let (s, e) = two_sum(self.hi, x);
        let (h, l) = two_sum(s, self.lo + e);
        self.hi = h;
        self.lo = l;
    }
    fn add_dd(&mut self, x: Dd) {
        self.add(x.hi);
        self.add(x.lo);
    }
    fn neg(self) -> Dd {
        Dd { hi: -self.hi, lo: -self.lo }
    }
    fn value(self) -> f64 {
        self.hi + self.lo
    }
}

const BEAT: u64 = 500; // 2 ms steps per simulated second
const DAY: u64 = 43_200_000; // 2 ms steps per simulated day
const TAU: f64 = 2.0 * std::f64::consts::PI;

/// Column k's per-step weight: a pulsation of period `period` steps at phase k/6, plus jitter.
fn weight(step: u64, period: u64, k: u64) -> f64 {
    let phase = ((step % period) as f64 / period as f64) + k as f64 / 6.0;
    let jitter = ((step as f64) * (0.618_033_988_749_894_9 + 0.1 * k as f64)).fract() - 0.5;
    1.0 + 0.5 * (TAU * phase).sin() + 0.1 * jitter
}

/// Internal columns (from, to, extent) and the exterior (intake into 0, output from 2) for one
/// 2 ms step.
fn rates(step: u64, rho: f64, scale: f64) -> ([(usize, usize, f64); 5], f64, f64) {
    let ring = |k| rho * scale * weight(step, BEAT, k);
    let small = |k| rho * scale * 1e-6 * weight(step, BEAT, k);
    let kappa = rho * 1e-6 * scale;
    let intake = kappa * weight(step, DAY, 0);
    let output = kappa * (1.0 + 0.1 * (((step as f64) * 0.414_213_562_373_095_1).fract() - 0.5));
    ([(0, 1, ring(0)), (1, 2, ring(1)), (2, 0, ring(2)), (0, 3, small(3)), (3, 0, small(4))], intake, output)
}

struct Outcome {
    defect: f64,
    inventory: f64,
    exterior: f64,
    drift: f64,
    bits: f64,
}

/// Runs the four ledgers side by side on one stream of extents.
fn run(core: &str, rho: f64, scale: f64) -> [Outcome; 4] {
    let init = [scale, scale, scale, scale * 1e-6];
    let (macro_steps, bins) = if core == "beat" { (172_800u64, BEAT) } else { (86_400_000u64, 1) };
    let quantum = scale * 2f64.powi(-56);
    let mut inventory0 = Dd::default();
    init.iter().for_each(|&x| inventory0.add(x));
    let mut plain = init;
    let mut dd = init.map(Dd::of);
    let mut fixed = init.map(|x| (x / quantum).round() as i64);
    let fixed0: i64 = fixed.iter().sum();
    let mut carried = fixed;
    let mut carry = [0.0f64; 7]; // five internal columns, intake, output
    let (mut in_c, mut out_c) = (0i64, 0i64);
    let mut quantize = |slot: usize, e: f64| -> i64 {
        let k = ((e + carry[slot]) / quantum).round() as i64;
        carry[slot] += e - k as f64 * quantum;
        k
    };
    let (mut in_f, mut out_f) = (0.0f64, 0.0f64);
    let (mut in_d, mut out_d) = (Dd::default(), Dd::default());
    let (mut in_q, mut out_q) = (0i64, 0i64);
    let mut exterior = Dd::default();
    let mut peak = 0i64;
    for m in 0..macro_steps {
        // A macro step's extents are f64 sums of its bins; a 2 ms core has one bin per step.
        let mut cols = [(0usize, 0usize, 0.0f64); 5];
        let (mut intake, mut output) = (0.0, 0.0);
        for b in 0..bins {
            let (c, i, o) = rates(m * bins + b, rho, scale);
            for (acc, col) in cols.iter_mut().zip(c) {
                *acc = (col.0, col.1, acc.2 + col.2);
            }
            intake += i;
            output += o;
        }
        exterior.add(intake);
        exterior.add(output);
        for (slot, (from, to, e)) in cols.into_iter().enumerate() {
            plain[from] -= e;
            plain[to] += e;
            dd[from].add(-e);
            dd[to].add(e);
            let k = (e / quantum).round() as i64;
            fixed[from] -= k;
            fixed[to] += k;
            let kc = quantize(slot, e);
            carried[from] -= kc;
            carried[to] += kc;
        }
        plain[0] += intake;
        plain[2] -= output;
        in_f += intake;
        out_f += output;
        dd[0].add(intake);
        dd[2].add(-output);
        in_d.add(intake);
        out_d.add(output);
        let ki = (intake / quantum).round() as i64;
        let ko = (output / quantum).round() as i64;
        fixed[0] += ki;
        fixed[2] -= ko;
        in_q += ki;
        out_q += ko;
        let (kci, kco) = (quantize(5, intake), quantize(6, output));
        carried[0] += kci;
        carried[2] -= kco;
        in_c += kci;
        out_c += kco;
        peak = peak.max(*fixed.iter().max().unwrap()).max(*carried.iter().max().unwrap());
    }
    let mut d_plain = Dd::default();
    plain.iter().for_each(|&x| d_plain.add(x));
    d_plain.add_dd(inventory0.neg());
    d_plain.add(-in_f);
    d_plain.add(out_f);
    let mut d_dd = Dd::default();
    dd.iter().for_each(|&x| d_dd.add_dd(x));
    d_dd.add_dd(inventory0.neg());
    d_dd.add_dd(in_d.neg());
    d_dd.add_dd(out_d);
    let end: i64 = fixed.iter().sum();
    let d_fixed = ((end - fixed0) - (in_q - out_q)) as f64 * quantum;
    let end_c: i64 = carried.iter().sum();
    let d_carried = ((end_c - fixed0) - (in_c - out_c)) as f64 * quantum;
    // Drift: largest pool distance from the double-double pool, in the same units.
    let drift = |pools: &[i64; 4]| {
        (0..4).map(|i| {
            let mut d = dd[i].neg();
            d.add(pools[i] as f64 * quantum);
            d.value().abs()
        }).fold(0.0f64, f64::max)
    };
    let bits = (peak as f64).log2();
    let outcome = |defect: f64, drift: f64, bits: f64| Outcome {
        defect,
        inventory: inventory0.value(),
        exterior: exterior.value(),
        drift,
        bits,
    };
    [
        outcome(d_plain.value(), 0.0, 0.0),
        outcome(d_dd.value(), 0.0, 0.0),
        outcome(d_fixed, drift(&fixed), bits),
        outcome(d_carried, drift(&carried), bits),
    ]
}

fn main() {
    println!("core  mode  rho    scale  |D|/inventory  |D|/exterior  drift/exterior  bits used");
    for core in ["2ms", "beat"] {
        for rho in [1e-3, 1e-6, 1e-9, 1e-12, 1e-15] {
            for scale in [1.0, 1e-12] {
                for (mode, o) in ["f64", "dd", "i64", "i64c"].into_iter().zip(run(core, rho, scale)) {
                    let (r, b) = if mode.starts_with("i64") {
                        (format!("{:.2e}", o.drift / o.exterior), format!("{:.1}", o.bits))
                    } else {
                        ("-".into(), "-".into())
                    };
                    println!(
                        "{core:<5} {mode:<5} {rho:<6.0e} {scale:<6.0e} {:<14.2e} {:<13.2e} {r:<15} {b}",
                        o.defect.abs() / o.inventory,
                        o.defect.abs() / o.exterior
                    );
                }
            }
        }
    }
}
