mod link_log;

use core::num::NonZeroU64;
use link_log::LinkLog;
use ng_contract::*;
use ng_record::Session;
use ng_validate::*;
use std::hint::black_box;
use std::time::Instant;

const MS: u64 = 1_000_000;
const UPDATES: u32 = 2_000;

fn peak_kb() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("the process status");
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|kb| kb.trim().strip_suffix("kB"))
        .and_then(|kb| kb.trim().parse().ok())
        .expect("a VmHWM line in kB")
}

fn reset_peak() {
    std::fs::write("/proc/self/clear_refs", "5").expect("the kernel resets the peak resident set");
}

fn hex(digest: Digest) -> String {
    digest.as_bytes()[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn solver(named: &str, interval_ms: u64) -> Box<dyn CoupledSolver> {
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    match named {
        "reference" => Box::new(
            ReferenceIntegrator::new(interval_ms * MS)
                .with_floor(NonZeroU64::new(MS / 10).expect("a floor"))
                .with_scales(ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive")),
        ),
        "ng-solver" => Box::new(ng_solver::Core::new(interval_ms * MS)),
        other => panic!("no core named {other}"),
    }
}

fn newton_update_us(body: &CompiledBody) -> f64 {
    let (packet, lens) = (
        arena_packet(body).expect("packet"),
        arena_lens(body).expect("lens"),
    );
    let parameters = body.bind(packet).expect("bound");
    let port = BoundaryPort::new(Vec::new()).expect("port");
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let system = body.initialization(parameters, port, request).expect("system");
    let workspace = system.workspace();
    let mut update = Vec::new();
    let started = Instant::now();
    for _ in 0..UPDATES {
        let (residuals, block) = system.stage(&workspace, 0).expect("stage");
        let factors = body.factor(&block).expect("factor");
        update = residuals;
        factors.solve(&mut update);
        black_box(&update);
    }
    let seconds = started.elapsed().as_secs_f64();
    assert!(
        update.iter().all(|change| change.is_finite()),
        "the last update is not finite"
    );
    1e6 * seconds / f64::from(UPDATES)
}

fn main() {
    let interval_ms: u64 = std::env::args()
        .nth(1)
        .and_then(|given| given.parse().ok())
        .expect("a step interval in ms");
    let named = std::env::args().nth(2).unwrap_or_else(|| "reference".to_string());
    let simulated_s: u64 = std::env::args()
        .nth(3)
        .map_or(Some(5), |given| given.parse().ok())
        .expect("simulated seconds");
    let started = Instant::now();
    let body = arena_body().expect("compiled");
    println!("time compile_ms {:.1}", 1e3 * started.elapsed().as_secs_f64());
    println!("work rows {}", body.rows().len());
    println!("work columns {}", body.columns().len());
    println!("work longest_step_ns {}", body.longest_step_ns());
    println!("time newton_update_us {:.1}", newton_update_us(&body));

    let core = solver(&named, interval_ms);
    println!("work core {named}");
    let (packet, lens) = (
        arena_packet(&body).expect("packet"),
        arena_lens(&body).expect("lens"),
    );
    let port = BoundaryPort::new(Vec::new()).expect("port");
    let log = LinkLog::create();
    let started = Instant::now();
    let admitted =
        admit_at(&body, packet, port, core.as_ref(), &lens, anchor_moment(), &log).expect("admitted");
    println!("time admission_s {:.2}", started.elapsed().as_secs_f64());
    println!("memory admission_peak_kb {}", peak_kb());
    reset_peak();

    let origin = anchor_moment().nanoseconds();
    let started = Instant::now();
    let reached = run_until(
        admitted.session,
        SimTime::from_ns(origin + simulated_s * 1_000 * MS),
    );
    let seconds = started.elapsed().as_secs_f64();
    let session = reached.unwrap_or_else(|stopped| panic!("fault: {:?}", stopped.fault()));
    let record = session.record();
    let simulated_ns = record.accepted().time().nanoseconds() - origin;
    println!("work accepted_steps {}", record.revision());
    println!("work simulated_ns {simulated_ns}");
    println!("work seal {}", hex(record.seal().digest()));
    println!("time x_real_time {:.3}", simulated_ns as f64 / 1e9 / seconds);
    println!("time ms_a_step {:.3}", 1e3 * seconds / record.revision() as f64);
    println!("memory run_peak_kb {}", peak_kb());
    println!("work history_bytes_a_step {}", log.bytes() / record.revision());

    let saved = session.checkpoint().expect("a checkpoint");
    let identity = record.identity().clone();
    drop(session);
    let fresh = solver(&named, interval_ms);
    reset_peak();
    let resumed = Session::resume(&saved, &identity, &body, fresh.as_ref(), &log).expect("resumed");
    println!("memory resume_peak_kb {}", peak_kb());
    println!("work resumed_seal {}", hex(resumed.record().seal().digest()));
}
