"""The generative physiology study's command.

    python3 -I tools/generative-physiology/gp.py audit --data DATA --source-commit C --workers N [--disk PATH ...]
    python3 -I tools/generative-physiology/gp.py audit --data DATA --source-commit C --report
    python3 -I tools/generative-physiology/gp.py selftest [--real GP_DRIVER]

Python 3.11 standard library only. `audit` makes the teacher audit's runs through $GP_DRIVER into DATA/audit and is
safe to repeat; `--report` reads the settled runs and prints the results template, starting no process. `selftest`
runs the study's tests on synthetic fixtures in seconds; `--real` adds a short pass through a built driver.

Exit 0: done (a failed gate is a result). 1: refused or a driver failed. 2: usage, or a report over an unfinished
audit.
"""
import pathlib
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))


def main(argv: list[str]) -> int:
    match argv:
        case ["selftest"]:
            import selftest

            return selftest.run(None)
        case ["selftest", "--real", driver]:
            import selftest

            return selftest.run(pathlib.Path(driver))
        case ["audit", *rest]:
            import audit

            return audit.command(rest)
        case _:
            print(__doc__, file=sys.stderr)
            return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
