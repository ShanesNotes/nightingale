"""The gate's policy: which steps and crates a run covers, and the workspace rules 05-seam holds.

Python 3.11 stdlib only. Run as `python3 -I tools/gate/gate.py <command>`; it finds the repo from its own path.

    plan <merge|changed|full> [--base REV]   the run's plan as key=value lines

Exit codes: 0 pass; 1 violations, one per line as `<code>: <subject>: <what to do>`; 2 an input the gate
reads is missing or malformed.
"""

from __future__ import annotations

import dataclasses
import json
import os
import pathlib
import re
import stat
import subprocess
import sys
import tomllib
from collections.abc import Callable, Mapping
from typing import Literal

Mode = Literal["merge", "changed", "full"]
Scope = Literal["docs-only", "affected", "everything"]
Phase = Literal["registration", "seam"]

ROOT = pathlib.Path(__file__).resolve().parents[2]
PATHS = "tools/gate/paths.toml"
STEPS = "tools/verify.d"
REGULAR = "100644"
BUILD_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "clippy.toml", "rustfmt.toml")


class GateError(Exception):
    """An input the gate reads is missing or malformed. The run fails; it is never a pass."""


@dataclasses.dataclass(frozen=True)
class Violation:
    code: str
    subject: str
    fix: str

    def __str__(self) -> str:
        return f"{self.code}: {self.subject}: {self.fix}"


class Violations(Exception):
    def __init__(self, found: list[Violation]):
        self.found = found


# ---- tools/gate/paths.toml ----------------------------------------------------------------------------


@dataclasses.dataclass(frozen=True)
class Glob:
    """`*` stays inside one path segment, `**` crosses segments and `**/` may match no directory."""

    text: str
    regex: re.Pattern[str]

    @classmethod
    def parse(cls, text: str) -> Glob:
        out, i = [], 0
        while i < len(text):
            if text.startswith("**/", i):
                out.append("(?:.*/)?")
                i += 3
            elif text.startswith("**", i):
                out.append(".*")
                i += 2
            elif text[i] == "*":
                out.append("[^/]*")
                i += 1
            else:
                out.append(re.escape(text[i]))
                i += 1
        return cls(text, re.compile("".join(out)))

    def matches(self, path: str) -> bool:
        return self.regex.fullmatch(path) is not None


@dataclasses.dataclass(frozen=True)
class PathRule:
    globs: tuple[Glob, ...]
    run: Literal["everything", "crate"] | tuple[str, ...]
    docs_only: bool

    def matches(self, path: str) -> bool:
        return any(g.matches(path) for g in self.globs)


@dataclasses.dataclass(frozen=True)
class F64Allow:
    item: str
    reason: str


@dataclasses.dataclass(frozen=True)
class CrateRow:
    name: str
    smoke: tuple[str, ...] = ()
    dev_external: Mapping[str, str] = dataclasses.field(default_factory=dict)
    f64_allow: tuple[F64Allow, ...] = ()


@dataclasses.dataclass(frozen=True)
class Registry:
    always: tuple[str, ...]
    crate_steps: tuple[str, ...]
    rules: tuple[PathRule, ...]
    crates: tuple[CrateRow, ...]

    def rule_for(self, path: str) -> PathRule | None:
        return next((r for r in self.rules if r.matches(path)), None)

    def row(self, name: str) -> CrateRow | None:
        return next((r for r in self.crates if r.name == name), None)

    def named_steps(self) -> frozenset[str]:
        named = set(self.always) | set(self.crate_steps)
        for r in self.rules:
            if isinstance(r.run, tuple):
                named |= set(r.run)
        return frozenset(named)


def _table(raw: object, where: str, required: set[str], optional: set[str] = frozenset()) -> dict:
    if not isinstance(raw, dict):
        raise GateError(f"{where}: expected a table")
    if missing := required - raw.keys():
        raise GateError(f"{where}: missing {', '.join(sorted(missing))}")
    if unknown := raw.keys() - required - optional:
        raise GateError(f"{where}: unknown key {', '.join(sorted(unknown))}")
    return raw


def _strings(raw: object, where: str) -> tuple[str, ...]:
    if not isinstance(raw, list) or not all(isinstance(s, str) and s for s in raw):
        raise GateError(f"{where}: expected a list of names")
    return tuple(raw)


def parse_registry(text: str) -> Registry:
    try:
        raw = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        raise GateError(f"{PATHS}: {e}") from None
    _table(raw, PATHS, {"schema", "always", "crate_steps"}, {"rule", "crate"})
    if raw["schema"] != 1:
        raise GateError(f"{PATHS}: schema {raw['schema']!r}; this gate reads schema 1")
    rules = []
    for i, r in enumerate(raw.get("rule", [])):
        where = f"{PATHS}: rule {i + 1}"
        _table(r, where, {"paths", "run"}, {"docs_only"})
        run = r["run"]
        if run not in ("everything", "crate"):
            run = _strings(run, f"{where}: run")
        docs_only = r.get("docs_only", False)
        if not isinstance(docs_only, bool):
            raise GateError(f"{where}: docs_only must be true or false")
        rules.append(PathRule(tuple(Glob.parse(g) for g in _strings(r["paths"], f"{where}: paths")), run, docs_only))
    crates = []
    for i, c in enumerate(raw.get("crate", [])):
        where = f"{PATHS}: crate {i + 1}"
        _table(c, where, {"name"}, {"smoke", "dev_external", "f64_allow"})
        dev = c.get("dev_external", {})
        if not isinstance(dev, dict) or not all(
            isinstance(v, str) and re.fullmatch(r"=\d+\.\d+\.\d+", v) for v in dev.values()
        ):
            raise GateError(f"{where}: dev_external maps a crate name to an exact requirement such as \"=1.5.1\"")
        allows = []
        for j, a in enumerate(c.get("f64_allow", [])):
            a = _table(a, f"{where}: f64_allow {j + 1}", {"item", "reason"})
            if not all(isinstance(a[k], str) and a[k].strip() for k in ("item", "reason")):
                raise GateError(f"{where}: f64_allow {j + 1}: item and reason must be non-empty")
            allows.append(F64Allow(a["item"], a["reason"]))
        crates.append(
            CrateRow(c["name"], _strings(c.get("smoke", []), f"{where}: smoke"), dict(dev), tuple(allows))
        )
    return Registry(
        _strings(raw["always"], f"{PATHS}: always"),
        _strings(raw["crate_steps"], f"{PATHS}: crate_steps"),
        tuple(rules),
        tuple(crates),
    )


# ---- the workspace ------------------------------------------------------------------------------------


@dataclasses.dataclass(frozen=True)
class Dep:
    name: str  # the package name, whatever the manifest calls the dependency
    kind: Literal["normal", "dev", "build"]
    member: str | None  # the workspace member it resolves to; None for a crate from outside the workspace
    source: str | None
    req: str
    optional: bool


@dataclasses.dataclass(frozen=True)
class Target:
    name: str
    kinds: frozenset[str]
    src: str  # repo-relative when inside the repo


@dataclasses.dataclass(frozen=True)
class Member:
    name: str
    dir: str  # repo-relative manifest directory
    deps: tuple[Dep, ...]
    targets: tuple[Target, ...]
    manifest: Mapping[str, object]


@dataclasses.dataclass(frozen=True)
class Workspace:
    """Everything a rule reads, built from the real tree or from a probe's mutated copy of it."""

    root: pathlib.Path
    files: frozenset[str]
    steps: frozenset[str]
    registry: Registry
    members: Mapping[str, Member]
    root_manifest: Mapping[str, object]

    def owner(self, path: str) -> str | None:
        owners = [m for m in self.members.values() if path.startswith(m.dir + "/")]
        return max(owners, key=lambda m: len(m.dir)).name if owners else None


def _read(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as e:
        raise GateError(f"{path}: {e.strerror}") from None


def _toml(path: pathlib.Path) -> dict:
    try:
        return tomllib.loads(_read(path))
    except tomllib.TOMLDecodeError as e:
        raise GateError(f"{path}: {e}") from None


def _rel(root: pathlib.Path, path: str | pathlib.Path) -> str:
    p = pathlib.Path(path).resolve()
    return p.relative_to(root.resolve()).as_posix() if p.is_relative_to(root.resolve()) else p.as_posix()


def git(root: pathlib.Path, *args: str) -> str | None:
    r = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def _git_z(root: pathlib.Path, *args: str) -> list[str]:
    r = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if r.returncode != 0:
        raise GateError(f"git {' '.join(args)}: {r.stderr.decode(errors='replace').strip()}")
    return [p for p in r.stdout.decode().split("\0") if p]


def list_files(root: pathlib.Path) -> frozenset[str]:
    """Tracked plus untracked files that git does not ignore; every file in a probe's copy, which has no .git."""
    if (root / ".git").exists():
        listed = _git_z(root, "ls-files", "-z", "--cached", "--others", "--exclude-standard")
        return frozenset(p for p in listed if os.path.lexists(root / p))
    return frozenset(
        (pathlib.Path(d) / f).relative_to(root).as_posix() for d, _, fs in os.walk(root) for f in fs
    )


def cargo_members(root: pathlib.Path) -> dict[str, Member]:
    """`--no-deps` reads the manifests without resolving, so it needs no network and no fresh lockfile."""
    r = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline", "--locked"],
        cwd=root, capture_output=True, text=True,
    )
    if r.returncode != 0:
        raise GateError(f"cargo metadata: {r.stderr.strip()}")
    meta = json.loads(r.stdout)
    ids = set(meta["workspace_members"])
    packages = [p for p in meta["packages"] if p["id"] in ids]
    by_dir = {pathlib.Path(p["manifest_path"]).parent.resolve(): p["name"] for p in packages}
    members = {}
    for p in packages:
        deps = tuple(
            Dep(
                d["name"],
                d["kind"] or "normal",
                by_dir.get(pathlib.Path(d["path"]).resolve()) if d.get("path") else None,
                d["source"],
                d["req"],
                d["optional"],
            )
            for d in p["dependencies"]
        )
        targets = tuple(Target(t["name"], frozenset(t["kind"]), _rel(root, t["src_path"])) for t in p["targets"])
        manifest = pathlib.Path(p["manifest_path"])
        members[p["name"]] = Member(p["name"], _rel(root, manifest.parent), deps, targets, _toml(manifest))
    return members


def load(root: pathlib.Path) -> Workspace:
    return Workspace(
        root=root,
        files=list_files(root),
        steps=frozenset(p.stem for p in (root / STEPS).glob("*.sh")),
        registry=parse_registry(_read(root / PATHS)),
        members=cargo_members(root),
        root_manifest=_toml(root / "Cargo.toml"),
    )


# ---- rules, each registered with the mutation that must make it fire -----------------------------------


@dataclasses.dataclass(frozen=True)
class Mutation:
    """One edit to a scratch copy of the files the rules read."""

    describe: str
    apply: Callable[[pathlib.Path], None]


@dataclasses.dataclass(frozen=True)
class Rule:
    code: str
    phase: Phase
    check: Callable[[Workspace], list[Violation]]
    probe: Mutation


RULES: list[Rule] = []


def rule(code: str, phase: Phase, probe: Mutation):
    """A rule cannot be registered without the mutation it must catch."""

    def register(check: Callable[[Workspace], list[Violation]]):
        RULES.append(Rule(code, phase, check, probe))
        return check

    return register


def check(ws: Workspace, phase: Phase) -> list[Violation]:
    return [v for r in RULES if r.phase == phase for v in r.check(ws)]


def edit(path: str, pattern: str, repl: str) -> Mutation:
    def apply(root: pathlib.Path) -> None:
        f = root / path
        text, n = re.subn(pattern, repl, f.read_text(encoding="utf-8"), count=1, flags=re.M)
        if n != 1:
            raise GateError(f"probe: {path} has no match for {pattern!r}; update the probe with the file")
        f.write_text(text, encoding="utf-8")

    return Mutation(f"edit {path}", apply)


def append(path: str, text: str) -> Mutation:
    def apply(root: pathlib.Path) -> None:
        f = root / path
        f.parent.mkdir(parents=True, exist_ok=True)
        with f.open("a", encoding="utf-8") as out:
            out.write(text)

    return Mutation(f"append to {path}", apply)


def add_member(name: str, row: bool) -> Mutation:
    def apply(root: pathlib.Path) -> None:
        d = root / "crates" / name
        (d / "src").mkdir(parents=True)
        (d / "Cargo.toml").write_text(
            f'[package]\nname = "{name}"\nversion.workspace = true\nedition.workspace = true\n\n'
            '[dependencies]\nng-contract = { path = "../ng-contract" }\n\n[lints]\nworkspace = true\n',
            encoding="utf-8",
        )
        (d / "src/lib.rs").write_text("//! A probe member.\n", encoding="utf-8")
        if row:
            append(PATHS, f'\n[[crate]]\nname = "{name}"\n').apply(root)

    return Mutation(f"add member {name}" + (" with a [[crate]] row" if row else ""), apply)


@rule("registration.rows", "registration", add_member("ng-solver", row=False))
def rows_match_members(ws: Workspace) -> list[Violation]:
    code = "registration.rows"
    names = [r.name for r in ws.registry.crates]
    out = [Violation(code, n, "two [[crate]] rows name it; keep one") for n in sorted({n for n in names if names.count(n) > 1})]
    for m in sorted(ws.members.values(), key=lambda m: m.name):
        if m.name not in names:
            out.append(Violation(code, m.name, f"member has no [[crate]] row; add one to {PATHS}"))
        if m.dir != f"crates/{m.name}":
            out.append(Violation(code, m.name, f"member lives at {m.dir}; a member lives at crates/{m.name}"))
    for n in sorted(set(names) - ws.members.keys()):
        out.append(Violation(code, n, f"[[crate]] row names no workspace member; add crates/{n} or drop the row"))
    return out


@rule("registration.steps", "registration", edit(PATHS, r"^always = \[", 'always = ["99-missing", '))
def steps_resolve(ws: Workspace) -> list[Violation]:
    code = "registration.steps"
    named = ws.registry.named_steps()
    out = [Violation(code, s, f"names no {STEPS}/{s}.sh; add the step or drop the name") for s in sorted(named - ws.steps)]
    out += [
        Violation(code, f"{STEPS}/{s}.sh", "no run would select it; name it in always, crate_steps or a rule")
        for s in sorted(ws.steps - named)
    ]
    for row in ws.registry.crates:
        m = ws.members.get(row.name)
        tests = {t.name for t in m.targets if "test" in t.kinds} if m else set()
        out += [Violation(code, f"{row.name}:{t}", "smoke names no test target of its crate") for t in row.smoke if m and t not in tests]
    return out


@rule("registration.docs-sound", "registration",
      append(PATHS, '\n[[rule]]\npaths = ["crates/**"]\nrun = ["10-docs"]\ndocs_only = true\n'))
def docs_globs_cover_no_code(ws: Workspace) -> list[Violation]:
    code = "registration.docs-sound"
    guarded = sorted({f for f in ws.files if ws.owner(f) or f.startswith("tools/")} | set(BUILD_FILES))
    out = []
    for r in ws.registry.rules:
        for g in r.globs if r.docs_only else ():
            if hit := next((f for f in guarded if g.matches(f)), None):
                out.append(Violation(code, g.text, f"a docs_only glob matches {hit}; narrow it so code is never docs-only"))
    return out


# ---- the plan -----------------------------------------------------------------------------------------


@dataclasses.dataclass(frozen=True)
class Plan:
    mode: Mode
    scope: Scope
    base: str | None
    reason: str
    crates: tuple[str, ...]
    smoke: tuple[str, ...]  # crate:test-target
    steps: tuple[str, ...]

    def lines(self) -> str:
        return "\n".join([
            f"mode={self.mode}", f"scope={self.scope}", f"base={self.base or '-'}", f"reason={self.reason}",
            f"crates={' '.join(self.crates)}", f"smoke={' '.join(self.smoke)}", f"steps={' '.join(self.steps)}",
        ])


Modes = tuple[str | None, str | None]  # git modes at the base and in the working tree; None where absent


def everything(ws: Workspace, mode: Mode, base: str | None, reason: str) -> Plan:
    smoke = () if mode == "changed" else tuple(f"{r.name}:{t}" for r in ws.registry.crates for t in r.smoke)
    return Plan(mode, "everything", base, reason, tuple(sorted(ws.members)), smoke, tuple(sorted(ws.steps)))


def reverse_closure(ws: Workspace, crates: set[str]) -> set[str]:
    dependents: dict[str, set[str]] = {}
    for m in ws.members.values():
        for d in m.deps:
            if d.member:
                dependents.setdefault(d.member, set()).add(m.name)
    seen, todo = set(crates), list(crates)
    while todo:
        for user in dependents.get(todo.pop(), ()):
            if user not in seen:
                seen.add(user)
                todo.append(user)
    return seen


def plan_changes(ws: Workspace, mode: Mode, base: str | None, changes: Mapping[str, Modes]) -> Plan:
    if not changes:
        return everything(ws, mode, base, "no change against the base")
    owners: set[str] = set()
    steps = set(ws.registry.always)
    docs_only = True
    for path in sorted(changes):
        r = ws.registry.rule_for(path)
        if r is None:
            return everything(ws, mode, base, f"{path}: no rule in {PATHS}")
        if r.run == "everything":
            return everything(ws, mode, base, f"{path}: runs everything")
        if r.docs_only and any(m != REGULAR for m in changes[path] if m is not None):
            return everything(ws, mode, base, f"{path}: a docs-only path that is not a regular file")
        docs_only &= r.docs_only
        if r.run == "crate":
            owner = ws.owner(path)
            if owner is None:
                return everything(ws, mode, base, f"{path}: no member owns it")
            owners.add(owner)
        else:
            steps |= set(r.run)
    crates = reverse_closure(ws, owners) if mode == "merge" else owners
    if crates:
        steps |= set(ws.registry.crate_steps)
    smoke = tuple(f"{r.name}:{t}" for r in ws.registry.crates for t in r.smoke) if mode == "merge" and crates else ()
    shown = sorted(changes)
    reason = shown[0] if len(shown) == 1 else f"{len(shown)} changed paths: {' '.join(shown[:3])}" + (" ..." if len(shown) > 3 else "")
    return Plan(mode, "docs-only" if docs_only else "affected", base, reason, tuple(sorted(crates)), smoke,
                tuple(sorted(steps)))


def resolve_base(root: pathlib.Path, arg: str | None) -> tuple[str | None, str]:
    """Never fetches: a stale origin/main only gives an older base and a wider run."""
    if arg is not None:
        if (sha := git(root, "rev-parse", "--verify", "--quiet", f"{arg}^{{commit}}")) is None:
            raise GateError(f"--base {arg}: not a commit")
        return sha, f"--base {arg}"
    main = git(root, "rev-parse", "--verify", "--quiet", "refs/remotes/origin/main^{commit}")
    if main is None:
        return None, "no base: refs/remotes/origin/main is absent"
    base = git(root, "merge-base", "HEAD", main)
    if base is None:
        return None, "no base: HEAD shares no history with origin/main"
    if base == git(root, "rev-parse", "HEAD"):
        parents = (git(root, "rev-list", "--parents", "-n", "1", "HEAD") or "").split()[1:]
        if len(parents) > 1:
            return parents[0], "HEAD is a merge already on origin/main; base HEAD^1"
        return None, "no base: HEAD is already on origin/main"
    return base, "merge-base with origin/main"


def _lstat_mode(path: pathlib.Path) -> str:
    st = os.lstat(path)
    if stat.S_ISLNK(st.st_mode):
        return "120000"
    if stat.S_ISREG(st.st_mode):
        return "100755" if st.st_mode & 0o111 else REGULAR
    return "040000"


def changed_paths(root: pathlib.Path, base: str) -> dict[str, Modes]:
    """The base against the working tree, so uncommitted edits count; a rename counts at both paths."""
    out: dict[str, Modes] = {}
    raw = _git_z(root, "diff", "--raw", "-z", "--no-renames", base)
    for meta, path in zip(raw[::2], raw[1::2]):
        old, new = meta.lstrip(":").split()[:2]
        out[path] = (None if old == "000000" else old, None if new == "000000" else new)
    for path in _git_z(root, "ls-files", "-z", "--others", "--exclude-standard"):
        out[path.rstrip("/")] = (None, _lstat_mode(root / path))
    return out


def plan(ws: Workspace, mode: Mode, base_arg: str | None) -> Plan:
    if found := check(ws, "registration"):
        raise Violations(found)
    if mode == "full":
        return everything(ws, mode, None, "the full run covers everything")
    base, why = resolve_base(ws.root, base_arg)
    if base is None:
        return everything(ws, mode, None, why)
    return plan_changes(ws, mode, base, changed_paths(ws.root, base))


# ---- plan cases: each expectation comes from the decided run semantics, not from a run ----------------


@dataclasses.dataclass(frozen=True)
class PlanCase:
    mode: Mode
    path: str
    scope: Scope
    crates: frozenset[str] | None = None  # None: not part of the expectation
    steps: frozenset[str] = frozenset()  # steps that must be selected
    modes: Modes = (REGULAR, REGULAR)


ALL4 = frozenset({"ng-contract", "ng-record", "ng-observe", "ng-validate"})
PLAN_CASES = (
    PlanCase("merge", "docs/srs.md", "docs-only", frozenset(), frozenset({"10-docs"})),
    PlanCase("merge", ".scratch/foundation/map.md", "docs-only", frozenset()),
    PlanCase("merge", "AGENTS.md", "docs-only", frozenset(), frozenset({"10-docs"})),
    PlanCase("merge", "crates/ng-record/src/lib.rs", "affected", frozenset({"ng-record", "ng-observe", "ng-validate"})),
    PlanCase("changed", "crates/ng-record/src/lib.rs", "affected", frozenset({"ng-record"})),
    PlanCase("merge", "crates/ng-contract/src/lib.rs", "affected", ALL4),
    PlanCase("merge", "Cargo.lock", "everything", ALL4),
    PlanCase("merge", "tools/verify", "everything", ALL4),
    PlanCase("merge", "notes.txt", "everything", ALL4),
    PlanCase("merge", "docs/srs.md", "everything", modes=(REGULAR, "100755")),
    PlanCase("merge", "AGENTS.md", "everything", modes=(REGULAR, "120000")),
)


def plan_cases(ws: Workspace) -> list[Violation]:
    out = []
    for c in PLAN_CASES:
        p = plan_changes(ws, c.mode, "base", {c.path: c.modes})
        subject = f"{c.mode} {c.path}" + ("" if c.modes == (REGULAR, REGULAR) else f" ({c.modes[1]})")
        if p.scope != c.scope:
            out.append(Violation("plan", subject, f"scope {p.scope}, the run semantics say {c.scope}"))
        if c.crates is not None and set(p.crates) != c.crates:
            out.append(Violation("plan", subject, f"crates {' '.join(p.crates) or 'none'}, expected {' '.join(sorted(c.crates)) or 'none'}"))
        if missing := c.steps - set(p.steps):
            out.append(Violation("plan", subject, f"does not select {' '.join(sorted(missing))}"))
        if c.scope == "docs-only" and set(p.steps) & set(ws.registry.crate_steps):
            out.append(Violation("plan", subject, "a docs-only plan selects a crate step"))
    return out


# ---- entry point --------------------------------------------------------------------------------------


def main(argv: list[str]) -> int:
    try:
        match argv:
            case ["plan", "merge" | "changed" | "full" as mode]:
                print(plan(load(ROOT), mode, None).lines())
            case ["plan", "merge" | "changed" as mode, "--base", base]:
                print(plan(load(ROOT), mode, base).lines())
            case _:
                print(__doc__, file=sys.stderr)
                return 2
    except Violations as e:
        for v in e.found:
            print(v)
        return 1
    except GateError as e:
        print(f"gate: {e}")
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
