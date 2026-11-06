"""Prints the engine's speed, or whether its resident memory stays flat as a run grows.

    python3 -I tools/bench/run.py [reference | ng-solver]
    python3 -I tools/bench/run.py memory [reference | ng-solver]

Python 3.11 stdlib only. A run is one process of src/main.rs: one admission of the arena body, then simulated
seconds on the named core (the reference by default), with no observer, then a resume of its checkpoint into a
fresh core. Each accepted step's link goes to a file under target/, so the history is on disk, not in memory.
Rounds in one process read about 0.7 percent slower than fresh processes, so each run gets its own. Exit 0
printed, 1 refused.

The default mode prints the median of 5 runs of 5 s at each step interval, with row count and machine load. The
interval is the reference's fixed step and ng-solver's longest step. A run takes about 25 s and the script about
4 minutes. The same head has moved 15 percent between hours, so compare two heads by taking turns in one sitting.

The memory mode runs the core's first interval for 5 s and for 50 s, 5 pairs taking turns, and executes the built
binary, so cargo's memory is not measured. It prints the medians of each length's peak resident sets: through
admission, through the run, and through the resume, each from a reset high-water mark. It says `flat: yes`, and
exits 0, when the run peaks' medians differ by at most MARGIN_KB.

What a run did (its `work` lines) must match across its length's or interval's runs, and only its `time` and
`memory` lines may differ. A fault, a killed run, a lost line, different work or a resume that lands on another
seal ends the script with no median. Each run's numbers print as it ends, so a killed script keeps them.

tools/verify never runs this: a wall clock cannot gate a merge, and the workspace bans one, so this project sits
outside it.
"""
import json
import math
import os
import pathlib
import statistics
import subprocess
import sys

RUNS = 5
INTERVALS_MS = {"reference": (5, 1), "ng-solver": (10, 5)}
SECONDS = (5, 50)
# Six times the 88 kB spread of run peaks measured on the tower, and 50 bytes a step over 10424 steps.
MARGIN_KB = 512
HERE = pathlib.Path(__file__).resolve().parent


def refuse(why):
    sys.exit(f"bench: {why}; no median")


def listing(label, measures):
    return f"{label}: " + ", ".join(f"{name} {value}" for name, value in measures.items())


def one_run(label, command):
    run = subprocess.run(command, cwd=HERE, stdout=subprocess.PIPE, text=True)
    if run.returncode != 0:
        refuse(f"the {label} run exited {run.returncode}")
    work, measured = {}, {}
    for line in run.stdout.splitlines():
        match line.split():
            case ["work", name, value]:
                work[name] = value
            case ["time" | "memory", name, value] if math.isfinite(float(value)) and float(value) > 0:
                measured[name] = float(value)
            case _:
                refuse(f"the {label} run printed {line!r}")
    if not work or not measured:
        refuse(f"the {label} run printed no result")
    if work.get("resumed_seal") != work.get("seal"):
        refuse(f"the {label} run resumed on seal {work.get('resumed_seal')} after {work.get('seal')}")
    return work, measured


def built():
    cargo = ["cargo", "build", "--release", "--locked", "--message-format=json-render-diagnostics"]
    build = subprocess.run(cargo, cwd=HERE, stdout=subprocess.PIPE, text=True)
    if build.returncode != 0:
        refuse("the build failed")
    for line in build.stdout.splitlines():
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message["target"]["name"] == "engine-bench":
            return message["executable"]
    refuse("the build named no engine-bench binary")


def taking_turns(labels, command):
    describe = ["git", "describe", "--always", "--dirty", "--abbrev=40"]
    head = subprocess.run(describe, cwd=HERE, stdout=subprocess.PIPE, text=True).stdout.strip()
    print(f"head {head}, release build, {os.cpu_count()} threads", flush=True)
    first, measures, loads = {}, {label: [] for label in labels}, []
    for number in range(1, RUNS + 1):
        for label in labels:
            loads.append(os.getloadavg()[0])
            work, measured = one_run(label, command(label))
            shape = (work, list(measured))
            if first.setdefault(label, shape) != shape:
                refuse(f"the {label} run {number} did different work: {shape} after {first[label]}")
            if number == 1:
                print(listing(label, work), flush=True)
            measures[label].append(measured)
            print(f"run {number} of {RUNS}, {listing(label, measured)}, load {loads[-1]:.2f}", flush=True)
    loads.append(os.getloadavg()[0])
    print(f"median of {RUNS} runs, low to high, load {min(loads):.2f} to {max(loads):.2f}")
    for label in labels:
        work, names = first[label]
        print(listing(label, work))
        for name in names:
            values = [measured[name] for measured in measures[label]]
            print(f"  {name} {statistics.median(values)} ({min(values)} to {max(values)})")
    return {label: first[label][0] for label in labels}, measures


def speed(core):
    built()
    labels = [f"{interval} ms" for interval in INTERVALS_MS[core]]
    cargo = ["cargo", "run", "--quiet", "--release", "--locked", "--"]
    taking_turns(labels, lambda label: cargo + [label.split()[0], core])


def memory(core):
    binary = built()
    interval = INTERVALS_MS[core][0]
    labels = [f"{seconds} s" for seconds in SECONDS]
    work, measures = taking_turns(labels, lambda label: [binary, str(interval), core, label.split()[0]])
    short, long = labels
    peaks = {label: [measured["run_peak_kb"] for measured in measures[label]] for label in labels}
    spread = max(max(values) - min(values) for values in peaks.values())
    steps = int(work[long]["accepted_steps"]) - int(work[short]["accepted_steps"])
    growth = statistics.median(peaks[long]) - statistics.median(peaks[short])
    print(f"accepted steps {work[short]['accepted_steps']} at {short}, {work[long]['accepted_steps']} at {long}")
    print(f"run peak medians differ by {growth} kB, margin {MARGIN_KB} kB, run-to-run spread {spread} kB")
    print(f"resident slope {round(1024 * growth / steps)} bytes a step over {steps} more accepted steps")
    print(f"history {work[long]['history_bytes_a_step']} bytes a step on disk")
    flat = abs(growth) <= MARGIN_KB
    print(f"flat: {'yes' if flat else 'no'}")
    sys.exit(0 if flat else 1)


def main():
    arguments = sys.argv[1:]
    mode = memory if arguments[:1] == ["memory"] else speed
    core = (arguments[1:] if mode is memory else arguments)[:1] or ["reference"]
    if core[0] not in INTERVALS_MS:
        refuse(f"no core named {core[0]!r}")
    mode(core[0])


if __name__ == "__main__":
    main()
