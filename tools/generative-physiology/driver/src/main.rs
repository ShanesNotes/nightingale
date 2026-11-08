use gp_driver::{
    IdentityDigests, LineLink, MemoryCap, Options, Outcome, RunSpec, RunState, SMOKE_SPEC, SpecId,
    TeachError, Teacher, check_smoke,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage:
  gp-driver smoke --out DIR [--source-commit SHA]
  gp-driver run SPEC --out DIR [--memory-cap-gb N] [--source-commit SHA]
  gp-driver interact SPEC --out DIR [--memory-cap-gb N] [--source-commit SHA]
  gp-driver status --out DIR --identity A0-SUMMARY SPEC...
interact writes one JSON line per beat on stdout and takes one answer line on stdin, none or an event line,
and reports on stderr. Exit 0: written or present. 1: an error or a live partial. 2: usage or a spec. 3: a fault.";

const DEFAULT_CAP_GB: u32 = 8;

enum Failure {
    Usage(String),
    Teach(TeachError),
}
impl From<TeachError> for Failure {
    fn from(error: TeachError) -> Self {
        Self::Teach(error)
    }
}
impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Self::Teach(TeachError::Io(error))
    }
}

enum Command {
    Smoke {
        out: PathBuf,
        options: Options,
    },
    Run {
        spec: PathBuf,
        out: PathBuf,
        options: Options,
        closed: bool,
    },
    Status {
        out: PathBuf,
        identity: PathBuf,
        specs: Vec<PathBuf>,
    },
}

fn command(arguments: &[String]) -> Result<Command, String> {
    let (verb, rest) = arguments.split_first().ok_or("no command")?;
    let mut positional = Vec::new();
    let mut flags = BTreeMap::new();
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        match word.strip_prefix("--") {
            Some(flag) => {
                let value = words.next().ok_or(format!("--{flag} takes a value"))?;
                if flags.insert(flag.to_string(), value.clone()).is_some() {
                    return Err(format!("--{flag} given twice"));
                }
            }
            None => positional.push(PathBuf::from(word)),
        }
    }
    let allowed: &[&str] = match verb.as_str() {
        "smoke" => &["out", "source-commit"],
        "run" | "interact" => &["out", "memory-cap-gb", "source-commit"],
        "status" => &["out", "identity"],
        other => return Err(format!("no command {other}")),
    };
    if let Some(flag) = flags.keys().find(|flag| !allowed.contains(&flag.as_str())) {
        return Err(format!("{verb} takes no --{flag}"));
    }
    let out = PathBuf::from(flags.get("out").ok_or("--out DIR is required")?);
    let cap = match flags.get("memory-cap-gb") {
        Some(given) => MemoryCap::gb(given.parse().map_err(|_| format!("--memory-cap-gb {given}"))?),
        None => MemoryCap::gb(DEFAULT_CAP_GB),
    };
    let source_commit = flags
        .get("source-commit")
        .cloned()
        .unwrap_or_else(|| "unrecorded".to_string());
    let options = Options { cap, source_commit };
    let closed = verb == "interact";
    match (verb.as_str(), positional.len()) {
        ("smoke", 0) => Ok(Command::Smoke { out, options }),
        ("run" | "interact", 1) => Ok(Command::Run {
            spec: positional.remove(0),
            out,
            options,
            closed,
        }),
        ("status", 1..) => Ok(Command::Status {
            out,
            identity: PathBuf::from(flags.get("identity").ok_or("--identity A0-SUMMARY is required")?),
            specs: positional,
        }),
        _ => Err(format!("{verb}: wrong arguments")),
    }
}

fn spec_at(path: &Path) -> Result<RunSpec, Failure> {
    let text =
        fs::read_to_string(path).map_err(|error| Failure::Usage(format!("{}: {error}", path.display())))?;
    RunSpec::parse(&text).map_err(|error| Failure::Usage(format!("{}: {error}", path.display())))
}

fn outcome(id: SpecId, outcome: &Outcome) -> (String, u8) {
    let (word, path, code) = match outcome {
        Outcome::Present(path) => ("present", path, 0),
        Outcome::Complete(path) => ("complete", path, 0),
        Outcome::FaultFound(path) | Outcome::Faulted(path) => ("fault", path, 3),
    };
    (format!("{word} {id} {}", path.display()), code)
}

fn run(spec: &Path, out: &Path, options: &Options, closed: bool) -> Result<u8, Failure> {
    let spec = spec_at(spec)?;
    let teacher = Teacher::new(spec.core())?;
    let result = if closed {
        teacher.interact(&spec, &mut LineLink::stdio(), options, out)?
    } else {
        teacher.run(&spec, options, out)?
    };
    let (line, code) = outcome(spec.id(), &result);
    if closed {
        eprintln!("{line}");
    } else {
        println!("{line}");
    }
    Ok(code)
}

fn smoke(out: &Path, options: &Options) -> Result<u8, Failure> {
    let spec =
        RunSpec::parse(SMOKE_SPEC).map_err(|error| Failure::Usage(format!("the smoke spec: {error}")))?;
    let teacher = Teacher::new(spec.core())?;
    let result = teacher.run(&spec, options, out)?;
    let (line, _) = outcome(spec.id(), &result);
    println!("{line}");
    let directory = match &result {
        Outcome::Present(path) | Outcome::Complete(path) => path,
        Outcome::FaultFound(_) | Outcome::Faulted(_) => {
            println!("smoke: failed: the run faulted");
            return Ok(1);
        }
    };
    print!("{}", fs::read_to_string(directory.join(gp_driver::SUMMARY))?);
    match check_smoke(directory) {
        Ok(checks) => {
            for check in checks {
                println!("smoke: {check}");
            }
            println!("smoke: passed");
            Ok(0)
        }
        Err(failure) => {
            println!("smoke: failed: {failure}");
            Ok(1)
        }
    }
}

fn status(out: &Path, identity: &Path, specs: &[PathBuf]) -> Result<u8, Failure> {
    let identity = IdentityDigests::read(identity)?;
    let mut code = 0;
    for path in specs {
        let spec = match spec_at(path) {
            Ok(spec) => spec,
            Err(Failure::Usage(error)) => {
                println!("invalid {error}");
                code = 2;
                continue;
            }
            Err(failure) => return Err(failure),
        };
        let id = spec.id();
        let state = RunState::of(out, id)?;
        let word = match &state {
            RunState::Present(made_by) if *made_by != identity => "stale",
            state => state.word(),
        };
        let held = matches!(state, RunState::Partial { held: true, .. });
        let suffix = if held { " held" } else { "" };
        println!("{word} {id} {}{suffix}", path.display());
    }
    Ok(code)
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match command(&arguments) {
        Err(error) => Err(Failure::Usage(error)),
        Ok(Command::Smoke { out, options }) => smoke(&out, &options),
        Ok(Command::Run {
            spec,
            out,
            options,
            closed,
        }) => run(&spec, &out, &options, closed),
        Ok(Command::Status { out, identity, specs }) => status(&out, &identity, &specs),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(Failure::Usage(error)) => {
            eprintln!("gp-driver: {error}\n{USAGE}");
            ExitCode::from(2)
        }
        Err(Failure::Teach(error)) => {
            eprintln!("gp-driver: {error}");
            ExitCode::from(1)
        }
    }
}
