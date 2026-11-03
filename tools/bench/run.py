"""Prints the engine's speed: the median of 5 runs at each step interval, with row count and machine load.

    python3 -I tools/bench/run.py [reference | ng-solver]

Python 3.11 stdlib only. A run is one process of src/main.rs: one admission of the arena body, then 5 simulated
seconds on the named core (the reference by default), with no observer. The interval is the reference's fixed step
and ng-solver's longest step. A run takes about 25 s and the script about 4 minutes. Rounds in
one process read about 0.7 percent slower than fresh processes, so each run gets its own. Exit 0 printed, 1 refused.

What a run did must match across its interval's runs, and only its times may differ. A fault, a killed run, a lost
line or different work ends the script with no median. Each run's numbers print as it ends, so a killed script
keeps them. The same head has moved 15 percent between hours, so compare two heads by taking turns in one sitting.

tools/verify never runs this: a wall clock cannot gate a merge, and the workspace bans one, so this project sits
outside it.
"""
import math
import os
import pathlib
import statistics
import subprocess
import sys

RUNS = 5
INTERVALS_MS = {"reference": (5, 1), "ng-solver": (10, 5)}
HERE = pathlib.Path(__file__).resolve().parent


def refuse(why):
    sys.exit(f"bench: {why}; no median")


def listing(interval, measures):
    return f"{interval} ms: " + ", ".join(f"{name} {value}" for name, value in measures.items())


def one_run(interval, core):
    cargo = ["cargo", "run", "--quiet", "--release", "--locked", "--", str(interval), core]
    run = subprocess.run(cargo, cwd=HERE, stdout=subprocess.PIPE, text=True)
    if run.returncode != 0:
        refuse(f"the {interval} ms run exited {run.returncode}")
    work, time = {}, {}
    for line in run.stdout.splitlines():
        match line.split():
            case ["work", name, value]:
                work[name] = value
            case ["time", name, value] if math.isfinite(float(value)) and float(value) > 0:
                time[name] = float(value)
            case _:
                refuse(f"the {interval} ms run printed {line!r}")
    if not work or not time:
        refuse(f"the {interval} ms run printed no result")
    return work, time


def main():
    core = sys.argv[1] if len(sys.argv) > 1 else "reference"
    if core not in INTERVALS_MS:
        refuse(f"no core named {core!r}")
    intervals = INTERVALS_MS[core]
    if subprocess.run(["cargo", "build", "--quiet", "--release", "--locked"], cwd=HERE).returncode != 0:
        refuse("the build failed")
    describe = ["git", "describe", "--always", "--dirty", "--abbrev=40"]
    head = subprocess.run(describe, cwd=HERE, stdout=subprocess.PIPE, text=True).stdout.strip()
    print(f"head {head}, release build, {os.cpu_count()} threads", flush=True)
    first, times, loads = {}, {interval: [] for interval in intervals}, []
    for number in range(1, RUNS + 1):
        for interval in intervals:
            loads.append(os.getloadavg()[0])
            work, time = one_run(interval, core)
            shape = (work, list(time))
            if first.setdefault(interval, shape) != shape:
                refuse(f"the {interval} ms run {number} did different work: {shape} after {first[interval]}")
            if number == 1:
                print(listing(interval, work), flush=True)
            times[interval].append(time)
            print(f"run {number} of {RUNS}, {listing(interval, time)}, load {loads[-1]:.2f}", flush=True)
    loads.append(os.getloadavg()[0])
    print(f"median of {RUNS} runs, low to high, load {min(loads):.2f} to {max(loads):.2f}")
    for interval in intervals:
        work, names = first[interval]
        print(listing(interval, work))
        for name in names:
            values = [time[name] for time in times[interval]]
            print(f"  {name} {statistics.median(values)} ({min(values)} to {max(values)})")


if __name__ == "__main__":
    main()
