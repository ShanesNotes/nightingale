"""The gate's policy: which steps and crates a run covers, and the workspace rules 05-seam holds.

Python 3.11 stdlib only. Run as `python3 -I tools/gate/gate.py <command>`; it finds the repo from its own path.

    plan <merge|changed|full> [--base REV]   the run's plan as key=value lines
    seam                                     the workspace rules, every rule's probe, the plan and scan cases
    bans                                     clippy rejects a probe built from clippy.toml, entry by entry
    f64                                      no bare f64 or f32 in a public API outside its row's allow list
    nightly-record <exit> <log>              record a nightly full run (tools/gate/nightly.sh)
    nightly-passed <sha>                     exit 0 when the nightly state records sha as passed
    doctests                                 every doctest rustdoc lists is one the host scan read

Exit codes: 0 pass; 1 violations, one per line as `<code>: <subject>: <what to do>`; 2 an input the gate
reads is missing or malformed.
"""

from __future__ import annotations

import dataclasses
import datetime
import hashlib
import json
import os
import pathlib
import posixpath
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import tomllib
from collections.abc import Callable, Mapping, Set
from typing import Literal

Mode = Literal["merge", "changed", "full"]
Scope = Literal["docs-only", "affected", "everything"]
Phase = Literal["registration", "seam"]

ROOT = pathlib.Path(__file__).resolve().parents[2]
PATHS = "tools/gate/paths.toml"
STEPS = "tools/verify.d"
CONTRACT = "docs/architecture/solver-contract.md"
REGULAR = "100644"
BUILD_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "clippy.toml", "rustfmt.toml")


class GateError(Exception):
    pass


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


@dataclasses.dataclass(frozen=True)
class Glob:
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


def _table(raw: object, where: str, required: Set[str], optional: Set[str] = frozenset()) -> dict:
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
            raise GateError(f'{where}: dev_external maps a crate name to an exact requirement such as "=1.5.1"')
        allows = []
        for j, a in enumerate(c.get("f64_allow", [])):
            a = _table(a, f"{where}: f64_allow {j + 1}", {"item", "reason"})
            if not all(isinstance(a[k], str) and a[k].strip() for k in ("item", "reason")):
                raise GateError(f"{where}: f64_allow {j + 1}: item and reason must be non-empty")
            allows.append(F64Allow(a["item"], a["reason"]))
        crates.append(CrateRow(c["name"], _strings(c.get("smoke", []), f"{where}: smoke"), dict(dev), tuple(allows)))
    return Registry(
        _strings(raw["always"], f"{PATHS}: always"),
        _strings(raw["crate_steps"], f"{PATHS}: crate_steps"),
        tuple(rules),
        tuple(crates),
    )


TABLE_HEADER = "| Module / proposed crate | Knowledge owned | Allowed body dependencies | Physiology closure |"
TABLE_NAME = re.compile(r"`(ng-[a-z0-9-]+)`( \(read only\))?")


@dataclasses.dataclass(frozen=True)
class TableRow:
    name: str
    allowed: frozenset[str]
    closure: bool


def _table_names(cell: str, where: str, read_only: bool) -> list[str]:
    names = []
    for part in cell.split(", "):
        m = TABLE_NAME.fullmatch(part)
        if not m or (m.group(2) and not read_only):
            raise GateError(f"{where}: {cell!r} is not a list of backticked crate names")
        names.append(m.group(1))
    return names


def parse_crate_table(text: str) -> dict[str, TableRow]:
    lines = text.splitlines()
    heads = [i for i, line in enumerate(lines) if line.strip() == TABLE_HEADER]
    if len(heads) != 1:
        raise GateError(f"{CONTRACT}: expected one crate table headed {TABLE_HEADER!r}, found {len(heads)}")
    rows: dict[str, TableRow] = {}
    i = heads[0] + 2
    while i < len(lines) and lines[i].startswith("|"):
        where = f"{CONTRACT}:{i + 1}"
        cells = [c.strip() for c in lines[i].strip().strip("|").split("|")]
        if len(cells) != 4:
            raise GateError(f"{where}: a crate table row has four cells")
        allowed = frozenset() if cells[2] == "None" else frozenset(_table_names(cells[2], where, read_only=True))
        if cells[3] not in ("yes", "no"):
            raise GateError(f"{where}: the physiology closure cell is yes or no")
        for name in _table_names(cells[0], where, read_only=False):
            if name in rows:
                raise GateError(f"{where}: {name} has two rows")
            rows[name] = TableRow(name, allowed, cells[3] == "yes")
        i += 1
    for r in rows.values():
        if unknown := r.allowed - rows.keys():
            raise GateError(f"{CONTRACT}: {r.name} may depend on {', '.join(sorted(unknown))}, which has no row")
    return rows


@dataclasses.dataclass(frozen=True)
class Token:
    kind: Literal["ident", "punct", "literal", "lifetime"]
    value: str
    line: int


@dataclasses.dataclass(frozen=True)
class Source:
    path: str
    code: tuple[Token, ...]
    doctests: tuple[Token, ...]
    blocks: tuple[int, ...]


class LexError(Exception):
    pass


RAW_STRING = re.compile(r'[bc]?r(#*)"')
QUOTED = re.compile(r'[bc]?"')


def _skip_quoted(text: str, i: int, quote: str) -> int:
    while i < len(text):
        if text[i] == "\\":
            i += 2
        elif text[i] == quote:
            return i + 1
        else:
            i += 1
    raise LexError("unterminated literal")


def lex(text: str, first_line: int = 1) -> tuple[list[Token], list[tuple[int, str]]]:
    tokens: list[Token] = []
    docs: list[tuple[int, str]] = []
    i, line, n = 0, first_line, len(text)
    while i < n:
        c = text[i]
        start_line = line
        if c == "\n":
            line += 1
            i += 1
        elif c.isspace():
            i += 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            body = text[i + 2 : j]
            if (body.startswith("/") and not body.startswith("//")) or body.startswith("!"):
                docs.append((line, body[1:]))
            i = j
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            if depth:
                raise LexError(f"line {line}: unterminated block comment")
            body = text[i + 2 : j - 2]
            if (body.startswith("*") and not body.startswith("**") and body != "*") or body.startswith("!"):
                docs.append((line, "\n".join(re.sub(r"^\s*\*? ?", "", ln) for ln in body[1:].split("\n"))))
            line += text.count("\n", i, j)
            i = j
        elif m := RAW_STRING.match(text, i):
            close = '"' + m.group(1)
            j = text.find(close, m.end())
            if j < 0:
                raise LexError(f"line {line}: unterminated raw string")
            tokens.append(Token("literal", text[m.end() : j], start_line))
            line += text.count("\n", i, j)
            i = j + len(close)
        elif m := QUOTED.match(text, i):
            j = _skip_quoted(text, m.end(), '"')
            tokens.append(Token("literal", text[m.end() : j - 1], start_line))
            line += text.count("\n", i, j)
            i = j
        elif text.startswith("b'", i) or (c == "'" and (text.startswith("\\", i + 1) or text.startswith("'", i + 2))):
            q = i + 1 if c == "b" else i
            j = _skip_quoted(text, q + 1, "'")
            tokens.append(Token("literal", text[q + 1 : j - 1], start_line))
            i = j
        elif c == "'":
            j = i + 1
            while j < n and (text[j] == "_" or text[j].isalnum()):
                j += 1
            tokens.append(Token("lifetime", text[i:j], start_line))
            i = j
        elif text.startswith("r#", i) and i + 2 < n and (text[i + 2] == "_" or text[i + 2].isalpha()):
            j = i + 2
            while j < n and (text[j] == "_" or text[j].isalnum()):
                j += 1
            tokens.append(Token("ident", text[i + 2 : j], start_line))
            i = j
        elif c == "_" or c.isalpha():
            j = i + 1
            while j < n and (text[j] == "_" or text[j].isalnum()):
                j += 1
            tokens.append(Token("ident", text[i:j], start_line))
            i = j
        elif c.isdigit():
            j = i + 1
            while j < n and (text[j] == "_" or text[j].isalnum() or (text[j] == "." and text[j + 1 : j + 2].isdigit())):
                j += 1
            tokens.append(Token("literal", text[i:j], start_line))
            i = j
        elif text.startswith("::", i):
            tokens.append(Token("punct", "::", start_line))
            i += 2
        else:
            tokens.append(Token("punct", c, start_line))
            i += 1
    return tokens, docs


def _punct(tokens: list[Token] | tuple[Token, ...], k: int, text: str) -> bool:
    return k < len(tokens) and tokens[k].kind == "punct" and tokens[k].value == text


def attributes(tokens: list[Token] | tuple[Token, ...]) -> list[list[Token]]:
    out, i = [], 0
    while i < len(tokens):
        j = i + 2 if _punct(tokens, i + 1, "!") else i + 1
        if _punct(tokens, i, "#") and _punct(tokens, j, "["):
            depth, k = 0, j
            while k < len(tokens):
                if tokens[k].kind == "punct" and tokens[k].value in ("(", "[", "{"):
                    depth += 1
                elif tokens[k].kind == "punct" and tokens[k].value in (")", "]", "}"):
                    depth -= 1
                    if depth == 0:
                        break
                k += 1
            out.append(list(tokens[j + 1 : k]))
            i = k
        i += 1
    return out


RUSTDOC_ATTRS = {
    "rust",
    "ignore",
    "should_panic",
    "no_run",
    "compile_fail",
    "test_harness",
    "standalone_crate",
}
FENCE = re.compile(r"^\s*(```+|~~~+)\s*(.*)$")


def _is_rust_fence(info: str) -> bool:
    words = [w for w in re.split(r"[\s,{}]+", info) if w and not re.fullmatch(r"edition.*|rust\d+", w)]
    known = [w for w in words if w in RUSTDOC_ATTRS or re.fullmatch(r"E\d{4}|ignore-.+", w)]
    return len(known) == len(words) or bool(known)


def _doc_groups(docs: list[tuple[int, str]]) -> list[list[tuple[int, str]]]:
    """Adjacent doc lines are one item's docs, which rustdoc unindents together."""
    groups: list[list[tuple[int, str]]] = []
    for first, text in docs:
        lines = [(first + k, ln) for k, ln in enumerate(text.split("\n"))]
        if groups and groups[-1][-1][0] + 1 == first:
            groups[-1] += lines
        else:
            groups.append(lines)
    return groups


def rust_blocks(docs: list[tuple[int, str]]) -> list[tuple[int, int, str]]:
    """The code blocks rustdoc runs: fenced blocks it reads as Rust, and lines indented four past the docs'
    own indent after a blank line. That second rule also takes a list item's indented paragraph, which
    rustdoc does not run: an over-reach, never a miss."""
    blocks = []
    for group in _doc_groups(docs):
        base = min((len(t) - len(t.lstrip()) for _, t in group if t.strip()), default=0)
        fence, rust, start, body, blank = None, False, 0, [], True
        for n, ln in group:
            indented = bool(ln.strip()) and len(ln) - len(ln.lstrip()) >= base + 4
            if fence is not None:
                if ln.strip().startswith(fence) and not ln.strip().strip(fence[0]):
                    if rust:
                        blocks.append((start - 1, start, "\n".join(body)))
                    fence, body = None, []
                else:
                    body.append(ln)
            elif body and (indented or not ln.strip()):
                body.append(ln)
            else:
                if body:
                    blocks.append((start, start, "\n".join(body)))
                    body = []
                if indented and blank:
                    start, body = n, [ln]
                elif m := FENCE.match(ln):
                    fence, start, rust = m.group(1), n + 1, _is_rust_fence(m.group(2))
            blank = not ln.strip()
        if body and (fence is None or rust):
            blocks.append((start - 1 if fence else start, start, "\n".join(body)))
    return blocks


def parse_source(path: str, text: str) -> Source:
    try:
        code, docs = lex(text)
    except LexError as e:
        raise GateError(f"{path}: cannot read as Rust: {e}") from None
    for attr in attributes(code):
        for k in range(len(attr) - 2):
            if (
                attr[k].kind == "ident"
                and attr[k].value == "doc"
                and _punct(attr, k + 1, "=")
                and attr[k + 2].kind == "literal"
            ):
                docs.append((attr[k].line, attr[k + 2].value.replace("\\n", "\n").replace('\\"', '"')))
    doctests: list[Token] = []
    blocks = rust_blocks(docs)
    for _, start, block in blocks:
        try:
            doctests += lex(block, start)[0]
        except LexError:
            doctests += [Token("ident", w, start) for w in re.findall(r"[A-Za-z_]\w*", block)]
    return Source(path, tuple(code), tuple(doctests), tuple(b[0] for b in blocks))


HOST_NAMES = frozenset({"std", "atomic", "env", "option_env", "thread_local"})
INCLUDE_NAMES = frozenset({"include", "include_str", "include_bytes"})
FILE_READERS: Mapping[str, frozenset[str]] = {"expect-test": frozenset({"expect_file"})}
TEST_ONLY = (["test"], ["not", "(", "test", ")"])


def host_hits(src: Source) -> list[Token]:
    return [
        t
        for t in (*src.code, *src.doctests)
        if t.kind == "ident" and (t.value in HOST_NAMES or re.fullmatch(r"is_\w+_feature_detected", t.value))
    ]


def _reads_home(code: tuple[Token, ...], k: int, src: Source, home: str) -> bool:
    """`expect_file!["x"]` reads x beside the calling file; only a literal that stays in the member is known."""
    if not (_punct(code, k + 1, "!") and k + 4 < len(code) and code[k + 3].kind == "literal"):
        return False
    if (code[k + 2].value, code[k + 4].value) not in (("[", "]"), ("(", ")"), ("{", "}")) or "\\" in code[k + 3].value:
        return False
    target = posixpath.normpath(posixpath.join(posixpath.dirname(src.path), code[k + 3].value))
    return target.startswith(home + "/")


def include_hits(src: Source, home: str) -> list[Token]:
    readers = INCLUDE_NAMES.union(*FILE_READERS.values())
    hits = [t for t in src.doctests if t.kind == "ident" and t.value in readers]
    hits += [
        t
        for k, t in enumerate(src.code)
        if t.kind == "ident"
        and t.value in readers
        and (t.value in INCLUDE_NAMES or not _reads_home(src.code, k, src, home))
    ]
    # A doctest compiles as its own crate, so a #[path] there reaches a file no scan reads.
    for attr in (*attributes(src.code), *attributes(src.doctests)):
        hits += [t for k, t in enumerate(attr) if t.kind == "ident" and t.value == "path" and _punct(attr, k + 1, "=")]
    return hits


def _predicate(toks: list[Token] | tuple[Token, ...], start: int, stop_at_comma: bool) -> list[str]:
    out, depth = [], 0
    for t in toks[start:]:
        if t.kind == "punct" and t.value in ("(", "[", "{"):
            depth += 1
        elif t.kind == "punct" and t.value in (")", "]", "}"):
            if depth == 0:
                break
            depth -= 1
        elif t.kind == "punct" and t.value == "," and depth == 0 and stop_at_comma:
            break
        out.append(t.value if t.kind != "literal" else '"..."')
    return out


def cfg_hits(src: Source) -> list[tuple[int, str]]:
    preds = []
    for attr in attributes(src.code):
        for k, t in enumerate(attr):
            if t.kind == "ident" and t.value in ("cfg", "cfg_attr") and _punct(attr, k + 1, "("):
                preds.append((t.line, _predicate(attr, k + 2, t.value == "cfg_attr")))
    code = src.code
    for k, t in enumerate(code):
        if t.kind == "ident" and t.value == "cfg" and _punct(code, k + 1, "!") and _punct(code, k + 2, "("):
            preds.append((t.line, _predicate(code, k + 3, False)))
    return [(line, " ".join(p)) for line, p in preds if p not in TEST_ONLY]


@dataclasses.dataclass(frozen=True)
class Dep:
    package: str
    kind: Literal["normal", "dev", "build"]
    member: str | None
    source: str | None
    req: str
    optional: bool


@dataclasses.dataclass(frozen=True)
class Target:
    name: str
    kinds: frozenset[str]
    src: pathlib.Path


@dataclasses.dataclass(frozen=True)
class Member:
    name: str
    dir: str
    deps: tuple[Dep, ...]
    targets: tuple[Target, ...]
    manifest: Mapping[str, object]
    sources: tuple[Source, ...]
    links: tuple[str, ...]


@dataclasses.dataclass(frozen=True)
class Workspace:
    root: pathlib.Path
    files: frozenset[str]
    steps: frozenset[str]
    registry: Registry
    table: Mapping[str, TableRow]
    members: Mapping[str, Member]
    root_manifest: dict

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
    if not p.is_relative_to(root.resolve()):
        raise GateError(f"{p} is outside the repo; a member lives at crates/<name>")
    return p.relative_to(root.resolve()).as_posix()


def git(root: pathlib.Path, *args: str) -> str | None:
    r = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else None


def _git_z(root: pathlib.Path, *args: str) -> list[str]:
    r = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if r.returncode != 0:
        raise GateError(f"git {' '.join(args)}: {r.stderr.decode(errors='replace').strip()}")
    return [p for p in r.stdout.decode().split("\0") if p]


def list_files(root: pathlib.Path) -> frozenset[str]:
    if (root / ".git").exists():
        listed = _git_z(root, "ls-files", "-z", "--cached", "--others", "--exclude-standard")
        return frozenset(p for p in listed if os.path.lexists(root / p))
    return frozenset((pathlib.Path(d) / f).relative_to(root).as_posix() for d, _, fs in os.walk(root) for f in fs)


def _member_tree(root: pathlib.Path, rel: str) -> tuple[tuple[Source, ...], tuple[str, ...]]:
    """Read from disk, ignored files included: rustc reads every module file whatever git ignores."""
    sources, links = [], []
    for d, dirs, files in os.walk(root / rel):
        for name in dirs + files:
            if (pathlib.Path(d) / name).is_symlink():
                links.append((pathlib.Path(d) / name).relative_to(root).as_posix())
        for name in files:
            p = pathlib.Path(d) / name
            if name.endswith(".rs") and not p.is_symlink():
                sources.append(parse_source(p.relative_to(root).as_posix(), _read(p)))
    return tuple(sorted(sources, key=lambda s: s.path)), tuple(sorted(links))


def cargo_members(root: pathlib.Path) -> dict[str, Member]:
    """`--no-deps` reads the manifests without resolving, so it needs no network and no fresh lockfile."""
    r = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline", "--locked"],
        cwd=root,
        capture_output=True,
        text=True,
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
        targets = tuple(
            Target(t["name"], frozenset(t["kind"]), pathlib.Path(t["src_path"]).resolve()) for t in p["targets"]
        )
        manifest = pathlib.Path(p["manifest_path"])
        rel = _rel(root, manifest.parent)
        members[p["name"]] = Member(p["name"], rel, deps, targets, _toml(manifest), *_member_tree(root, rel))
    return members


def load(root: pathlib.Path) -> Workspace:
    return Workspace(
        root=root,
        files=list_files(root),
        steps=frozenset(p.stem for p in (root / STEPS).glob("*.sh")),
        registry=parse_registry(_read(root / PATHS)),
        table=parse_crate_table(_read(root / CONTRACT)),
        members=cargo_members(root),
        root_manifest=_toml(root / "Cargo.toml"),
    )


@dataclasses.dataclass(frozen=True)
class Mutation:
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

    first = text.strip().splitlines()[0] if text.strip() else ""
    return Mutation(f"append {first!r} to {path}" if first else f"write {path}", apply)


def add_member(name: str, row: bool) -> Mutation:
    def apply(root: pathlib.Path) -> None:
        d = root / "crates" / name
        if d.exists():
            raise GateError(f"probe: crates/{name} is a real member; give the probe a name no crate will take")
        (d / "src").mkdir(parents=True)
        (d / "Cargo.toml").write_text(
            f'[package]\nname = "{name}"\nversion.workspace = true\nedition.workspace = true\n\n'
            '[dependencies]\nng-contract = { path = "../ng-contract" }\n\n[lints]\nworkspace = true\n',
            encoding="utf-8",
        )
        (d / "src/lib.rs").write_text("//! A probe member.\n", encoding="utf-8")
        if row:
            append(PATHS, f'\n[[crate]]\nname = "{name}"\n').apply(root)

    return Mutation(f"add member {name}" + (" with a [[crate]] row" if row else " with no [[crate]] row"), apply)


def add_outside_crate(name: str, to: str) -> Mutation:
    """The stub sits beside the copy, not inside it: Cargo makes a path
    dependency inside the workspace directory a member."""

    def apply(root: pathlib.Path) -> None:
        stub = root.parent / "outside" / name
        (stub / "src").mkdir(parents=True)
        (stub / "Cargo.toml").write_text(
            f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2024"\n', encoding="utf-8"
        )
        (stub / "src/lib.rs").write_text("", encoding="utf-8")
        rel = os.path.relpath(stub, root / "crates" / to)
        edit(
            f"crates/{to}/Cargo.toml", r"^\[dependencies\]\n", f'[dependencies]\n{name} = {{ path = "{rel}" }}\n'
        ).apply(root)

    return Mutation(f"{to} depends on a stub crate named {name} from outside the workspace", apply)


@rule("registration.rows", "registration", add_member("ng-unregistered", row=False))
def rows_match_members(ws: Workspace) -> list[Violation]:
    code = "registration.rows"
    names = [r.name for r in ws.registry.crates]
    out = [
        Violation(code, n, "two [[crate]] rows name it; keep one")
        for n in sorted({n for n in names if names.count(n) > 1})
    ]
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
    out = [
        Violation(code, s, f"names no {STEPS}/{s}.sh; add the step or drop the name") for s in sorted(named - ws.steps)
    ]
    out += [
        Violation(code, f"{STEPS}/{s}.sh", "no run would select it; name it in always, crate_steps or a rule")
        for s in sorted(ws.steps - named)
    ]
    for row in ws.registry.crates:
        m = ws.members.get(row.name)
        tests = {t.name for t in m.targets if "test" in t.kinds} if m else set()
        out += [
            Violation(code, f"{row.name}:{t}", "smoke names no test target of its crate")
            for t in row.smoke
            if m and t not in tests
        ]
    return out


@rule(
    "registration.docs-sound",
    "registration",
    append(PATHS, '\n[[rule]]\npaths = ["crates/**"]\nrun = ["10-docs"]\ndocs_only = true\n'),
)
def docs_globs_cover_no_code(ws: Workspace) -> list[Violation]:
    code = "registration.docs-sound"
    guarded = sorted({f for f in ws.files if ws.owner(f) or f.startswith("tools/")} | set(BUILD_FILES))
    out = []
    for r in ws.registry.rules:
        for g in r.globs if r.docs_only else ():
            if hit := next((f for f in guarded if g.matches(f)), None):
                out.append(
                    Violation(code, g.text, f"a docs_only glob matches {hit}; narrow it so code is never docs-only")
                )
    return out


@rule("seam.name", "seam", add_member("ng-unnamed", row=True))
def member_names(ws: Workspace) -> list[Violation]:
    return [
        Violation("seam.name", m, f"no row of the crate table in {CONTRACT} names it; a crate enters that table first")
        for m in sorted(ws.members)
        if m not in ws.table
    ]


@rule(
    "seam.edge",
    "seam",
    edit(
        "crates/ng-observe/Cargo.toml",
        r"^\[dependencies\]\n",
        '[dependencies]\nng-validate = { path = "../ng-validate" }\n',
    ),
)
def internal_edges(ws: Workspace) -> list[Violation]:
    out = []
    for m in sorted(ws.members.values(), key=lambda m: m.name):
        if (row := ws.table.get(m.name)) is None:
            continue
        allowed = ", ".join(sorted(row.allowed)) or "no workspace crate"
        out += [
            Violation(
                "seam.edge",
                f"{m.name} -> {d.member}",
                f"the crate table allows {allowed}; drop this {d.kind} dependency",
            )
            for d in m.deps
            if d.member and d.member not in row.allowed
        ]
    return out


@rule("seam.external", "seam", add_outside_crate("prost", to="ng-record"))
def no_outside_crates(ws: Workspace) -> list[Violation]:
    code, out = "seam.external", []
    for m in sorted(ws.members.values(), key=lambda m: m.name):
        if (row := ws.table.get(m.name)) is None:
            continue
        admitted = r.dev_external if (r := ws.registry.row(m.name)) else {}
        if row.closure and admitted:
            out.append(Violation(code, m.name, "a physiology closure crate admits no dev_external entry; drop it"))
        out += [
            Violation(
                code,
                f"{m.name} -> {p}",
                "the gate does not know which of its names read a file; add them to FILE_READERS in gate.py",
            )
            for p in sorted(admitted.keys() - FILE_READERS.keys())
        ]
        for d in m.deps:
            if d.member:
                continue
            if row.closure:
                out.append(
                    Violation(
                        code,
                        f"{m.name} -> {d.package}",
                        "the physiology closure takes no crate from outside the workspace, of any kind",
                    )
                )
            elif (
                d.kind != "dev"
                or admitted.get(d.package) != d.req
                or not (d.source or "").startswith(("registry+", "sparse+"))
            ):
                out.append(
                    Violation(
                        code,
                        f"{m.name} -> {d.package}",
                        f"a {d.kind} dependency at {d.req!r}; outside the closure a member takes only the "
                        "registry dev-dependencies its [[crate]] row's dev_external admits, at that exact version",
                    )
                )
    return out


@rule(
    "seam.closure",
    "seam",
    edit(CONTRACT, r"^(\| `ng-record` \|.*\| )`ng-contract`( \| yes \|)$", r"\1`ng-contract`, `ng-observe`\2"),
)
def closure_is_closed(ws: Workspace) -> list[Violation]:
    return [
        Violation(
            "seam.closure",
            f"{r.name} -> {a}",
            "the table marks it outside the physiology closure, which takes only closure crates",
        )
        for r in sorted(ws.table.values(), key=lambda r: r.name)
        if r.closure
        for a in sorted(r.allowed)
        if not ws.table[a].closure
    ]


@rule("seam.host", "seam", append("crates/ng-record/src/lib.rs", "use std::fs;\n"))
def no_host_service(ws: Workspace) -> list[Violation]:
    return [
        Violation(
            "seam.host",
            f"{s.path}:{t.line}",
            f"`{t.value}` names a host service (files, threads, the clock, the environment); "
            "no workspace crate may name it, even as an alias or a macro argument; rename or remove it",
        )
        for m in sorted(ws.members.values(), key=lambda m: m.name)
        for s in m.sources
        for t in host_hits(s)
    ]


@rule(
    "seam.cfg",
    "seam",
    append("crates/ng-contract/src/lib.rs", '#[cfg(feature = "hidden")]\npub fn leak(x: f64) -> f64 {\n    x\n}\n'),
)
def every_line_is_checked(ws: Workspace) -> list[Violation]:
    code, out = "seam.cfg", []
    for m in sorted(ws.members.values(), key=lambda m: m.name):
        if "features" in m.manifest:
            out.append(Violation(code, m.name, "a [features] table hides code from clippy, the tests and the f64 scan"))
        out += [
            Violation(code, f"{m.name} -> {d.package}", "an optional dependency is a feature; make it plain")
            for d in m.deps
            if d.optional
        ]
        out += [
            Violation(
                code,
                f"{s.path}:{line}",
                f"cfg predicate `{p}`; only test and not(test), so every line that can ship is checked",
            )
            for s in m.sources
            for line, p in cfg_hits(s)
        ]
    return out


@rule(
    "seam.include",
    "seam",
    append("crates/ng-record/src/lib.rs", 'const _: &str = include_str!("../../../README.md");\n'),
)
def sources_stay_home(ws: Workspace) -> list[Violation]:
    code, out = "seam.include", []
    for m in sorted(ws.members.values(), key=lambda m: m.name):
        out += [
            Violation(
                code,
                f"{s.path}:{t.line}",
                f"`{t.value}` reads a file at build or test time; a member builds and tests from its own sources only",
            )
            for s in m.sources
            for t in include_hits(s, m.dir)
        ]
        out += [
            Violation(code, link, "a symlink in a member can reach outside it; put the file there") for link in m.links
        ]
        out += [
            Violation(code, f"{m.name}:{t.name}", f"the target's source {t.src} is outside {m.dir}")
            for t in m.targets
            if not t.src.is_relative_to((ws.root / m.dir).resolve())
        ]
    return out


def lib_path(path: str) -> Mutation:
    def apply(root: pathlib.Path) -> None:
        append("crates/ng-record/Cargo.toml", f'\n[lib]\npath = "{path}"\n').apply(root)
        shutil.copy2(root / "crates/ng-record/src/lib.rs", root / "crates/ng-record" / path)

    return Mutation(f"point ng-record's [lib] path at {path}", apply)


@rule("seam.target", "seam", lib_path("src/lib.txt"))
def targets_are_scanned(ws: Workspace) -> list[Violation]:
    return [
        Violation("seam.target", f"{m.name}:{t.name}", f"{t.src.name} is not a .rs file, so no source scan reads it")
        for m in sorted(ws.members.values(), key=lambda m: m.name)
        for t in m.targets
        if t.src.suffix != ".rs"
    ]


@rule("seam.build", "seam", append("crates/ng-contract/build.rs", "fn main() {}\n"))
def no_build_code(ws: Workspace) -> list[Violation]:
    return [
        Violation(
            "seam.build", f"{m.name}:{t.name}", "a build script or proc macro runs host code at build time; remove it"
        )
        for m in sorted(ws.members.values(), key=lambda m: m.name)
        for t in m.targets
        if t.kinds & {"custom-build", "proc-macro"}
    ]


REQUIRED_LEVELS = (
    ("rust", "unsafe_code"),
    ("clippy", "disallowed_methods"),
    ("clippy", "disallowed_types"),
    ("clippy", "print_stdout"),
    ("clippy", "print_stderr"),
    ("clippy", "dbg_macro"),
)


@rule("seam.lints", "seam", edit("crates/ng-observe/Cargo.toml", r"^\[lints\]\nworkspace = true\n", ""))
def lints_inherited(ws: Workspace) -> list[Violation]:
    """A member without the table inherits nothing and nothing fails; forbid is the level no source
    attribute can lower."""
    out = [
        Violation("seam.lints", m.name, "a member's [lints] table is exactly `workspace = true`")
        for m in sorted(ws.members.values(), key=lambda m: m.name)
        if m.manifest.get("lints") != {"workspace": True}
    ]
    lints = ws.root_manifest.get("workspace", {}).get("lints", {})
    for group, name in REQUIRED_LEVELS:
        level = lints.get(group, {}).get(name)
        if (level.get("level") if isinstance(level, dict) else level) != "forbid":
            out.append(Violation("seam.lints", "Cargo.toml", f'[workspace.lints.{group}] {name} must be "forbid"'))
    return out


@rule("seam.clippy-conf", "seam", append("crates/ng-record/clippy.toml", "\n"))
def one_clippy_conf(ws: Workspace) -> list[Violation]:
    found = {f for f in ws.files if pathlib.PurePosixPath(f).name in ("clippy.toml", ".clippy.toml")}
    for m in ws.members.values():
        for d in (pathlib.PurePosixPath(m.dir), *pathlib.PurePosixPath(m.dir).parents):
            found |= {(d / n).as_posix() for n in ("clippy.toml", ".clippy.toml") if (ws.root / d / n).exists()}
    return [
        Violation(
            "seam.clippy-conf",
            f,
            "clippy reads only the nearest clippy.toml, so this one drops the root bans; delete it",
        )
        for f in sorted(found - {"clippy.toml"})
    ]


@rule("seam.cargo-config", "seam", append(".cargo/config.toml", "[build]\nrustflags = []\n"))
def no_cargo_config(ws: Workspace) -> list[Violation]:
    found = {f for f in ws.files if re.fullmatch(r"(?:.*/)?\.cargo/config(?:\.toml)?", f)}
    found |= {n for n in (".cargo/config", ".cargo/config.toml") if (ws.root / n).exists()}
    return [
        Violation(
            "seam.cargo-config", f, "a Cargo config can cap lints or swap rustc under every cargo call; delete it"
        )
        for f in sorted(found)
    ]


PROBE_COPY = ("Cargo.toml", "Cargo.lock", "clippy.toml", "rustfmt.toml", "rust-toolchain.toml", PATHS, CONTRACT)


def _copy(ws: Workspace, dest: pathlib.Path) -> None:
    for f in sorted(set(PROBE_COPY) | {f for f in ws.files if f.startswith("crates/")}):
        if os.path.lexists(ws.root / f):
            (dest / f).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ws.root / f, dest / f, follow_symlinks=False)
    (dest / STEPS).mkdir(parents=True, exist_ok=True)
    for s in ws.steps:
        (dest / STEPS / f"{s}.sh").touch()


def run_probes(ws: Workspace, tree_passes: bool) -> tuple[list[Violation], int]:
    out, bites = [], 0
    with tempfile.TemporaryDirectory(prefix="gate-probes-") as tmp:
        clean = pathlib.Path(tmp) / "clean/ws"
        _copy(ws, clean)
        try:
            copy_ws = load(clean)
        except GateError as e:
            return [
                Violation("probe", "the copy of the tree", f"does not load, so no probe ran: {str(e).splitlines()[0]}")
            ], 0
        if tree_passes:
            out += [
                Violation("probe", "the unmutated copy", f"fails {v}")
                for v in check(copy_ws, "registration") + check(copy_ws, "seam")
            ]
        for r in RULES:
            root = pathlib.Path(tmp) / r.code / "ws"
            _copy(ws, root)
            try:
                r.probe.apply(root)
                fired = any(v.code == r.code for v in r.check(load(root)))
            except GateError as e:
                out.append(Violation("probe", r.code, f"{r.probe.describe}: the mutated copy does not load: {e}"))
                continue
            if fired:
                bites += 1
            else:
                out.append(Violation("probe", r.code, f"{r.probe.describe}: the rule does not fire on it"))
    return out, bites


@dataclasses.dataclass(frozen=True)
class Plan:
    mode: Mode
    scope: Scope
    base: str | None
    reason: str
    crates: tuple[str, ...]
    smoke: tuple[str, ...]
    steps: tuple[str, ...]

    def lines(self) -> str:
        return "\n".join(
            [
                f"mode={self.mode}",
                f"scope={self.scope}",
                f"base={self.base or '-'}",
                f"reason={self.reason}",
                f"crates={' '.join(self.crates)}",
                f"smoke={' '.join(self.smoke)}",
                f"steps={' '.join(self.steps)}",
            ]
        )


Modes = tuple[str | None, str | None]


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
    reason = (
        shown[0]
        if len(shown) == 1
        else f"{len(shown)} changed paths: {' '.join(shown[:3])}" + (" ..." if len(shown) > 3 else "")
    )
    return Plan(
        mode, "docs-only" if docs_only else "affected", base, reason, tuple(sorted(crates)), smoke, tuple(sorted(steps))
    )


def resolve_base(root: pathlib.Path, arg: str | None) -> tuple[str | None, str]:
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
    out: dict[str, Modes] = {}
    raw = _git_z(root, "diff", "--raw", "-z", "--no-renames", base)
    for meta, path in zip(raw[::2], raw[1::2], strict=True):
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


@dataclasses.dataclass(frozen=True)
class PlanCase:
    mode: Mode
    path: str
    scope: Scope
    crates: frozenset[str] | None = None
    reaches: frozenset[str] = frozenset()
    steps: frozenset[str] = frozenset()
    modes: Modes = (REGULAR, REGULAR)


RECORD_USERS = frozenset({"ng-record", "ng-observe", "ng-validate"})
PLAN_CASES = (
    PlanCase("merge", "docs/srs.md", "docs-only", frozenset(), steps=frozenset({"10-docs"})),
    PlanCase("merge", ".scratch/foundation/map.md", "docs-only", frozenset()),
    PlanCase("merge", "AGENTS.md", "docs-only", frozenset(), steps=frozenset({"10-docs"})),
    PlanCase("merge", CONTRACT, "docs-only", frozenset(), steps=frozenset({"05-seam", "10-docs", "20-sketch"})),
    PlanCase(
        "merge", "docs/atlas/coupling-map.md", "docs-only", frozenset(), steps=frozenset({"10-docs", "20-sketch"})
    ),
    PlanCase("merge", "crates/ng-record/src/lib.rs", "affected", RECORD_USERS),
    PlanCase("changed", "crates/ng-record/src/lib.rs", "affected", frozenset({"ng-record"})),
    PlanCase("merge", "crates/ng-contract/src/lib.rs", "affected", reaches=RECORD_USERS | {"ng-contract"}),
    PlanCase("merge", "Cargo.lock", "everything"),
    PlanCase("merge", "tools/verify", "everything"),
    PlanCase("merge", "notes.txt", "everything"),
    PlanCase("merge", "docs/srs.md", "everything", modes=(REGULAR, "100755")),
    PlanCase("merge", "AGENTS.md", "everything", modes=(REGULAR, "120000")),
)


def plan_cases(ws: Workspace) -> list[Violation]:
    out = []
    for c in PLAN_CASES:
        p = plan_changes(ws, c.mode, "base", {c.path: c.modes})
        subject = f"{c.mode} {c.path}" + ("" if c.modes == (REGULAR, REGULAR) else f" ({c.modes[1]})")
        crates, steps = set(p.crates), set(p.steps)
        exact = set(ws.members) if c.scope == "everything" else c.crates
        if p.scope != c.scope:
            out.append(Violation("plan", subject, f"scope {p.scope}, the run semantics say {c.scope}"))
        if exact is not None and crates != exact:
            got, want = " ".join(sorted(crates)) or "none", " ".join(sorted(exact)) or "none"
            out.append(Violation("plan", subject, f"crates {got}, expected {want}"))
        if missing := c.reaches - crates:
            out.append(Violation("plan", subject, f"does not reach {' '.join(sorted(missing))}"))
        if missing := (ws.steps if c.scope == "everything" else c.steps) - steps:
            out.append(Violation("plan", subject, f"does not select {' '.join(sorted(missing))}"))
        if c.scope == "docs-only" and steps & set(ws.registry.crate_steps) - c.steps:
            out.append(Violation("plan", subject, "a docs-only plan selects a crate step"))
    return out


HOST, INCLUDE, CFG = frozenset({"seam.host"}), frozenset({"seam.include"}), frozenset({"seam.cfg"})
SCAN_CASES: tuple[tuple[str, frozenset[str]], ...] = (
    ("use std::fs;", HOST),
    ("// std::fs\n/* std /* nested */ std::fs */\nfn f() {}", frozenset()),
    ('const S: &str = "std::fs \\" std";', frozenset()),
    ('const S: &str = r#"std::fs " std"#;', frozenset()),
    ('const B: &[u8] = br##"std"##;', frozenset()),
    ("fn f<'a>(x: &'a str) -> char { 'a' }", frozenset()),
    ("const Q: char = '\\''; const S: &str = \"std\";", frozenset()),
    ('const C: u8 = b\'"\'; const S: &str = "std";', frozenset()),
    ("fn f() { let r#std = 1; }", HOST),
    ("use core::env as e;", HOST),
    ("use core::sync as s; fn f() -> usize { s::atomic::AtomicUsize::new(0).into_inner() }", HOST),
    ('macro_rules! r { ($r:ident) => { $r::fs::read("x") }; }\nfn f() { let _ = r!(std); }', HOST),
    ("thread_local! { static X: u8 = 0; }", HOST),
    ('fn f() -> bool { is_x86_feature_detected!("avx") }', HOST),
    ('/// ```\n/// std::fs::read("x").unwrap();\n/// ```\npub fn f() {}', HOST),
    ("/// ```no_run\n/// # use std::fs;\n/// ```\npub fn f() {}", HOST),
    ("/// ```text\n/// std::fs\n/// ```\npub fn f() {}", frozenset()),
    ('/// ```editionfoo\n/// std::fs::read("x");\n/// ```\npub fn f() {}', HOST),
    ('/// ```rust2021\n/// std::fs::read("x");\n/// ```\npub fn f() {}', HOST),
    ('#[doc = "```\\nstd::fs::read(\\"x\\");\\n```"]\npub fn f() {}', HOST),
    ('//! Text.\n//!\n//!     let _ = std::fs::read("x");\npub fn f() {}', HOST),
    ('/// Text.\n///     std::fs::read("x");\npub fn f() {}', frozenset()),
    ('///     std::fs::read("x");\npub fn f() {}', frozenset()),
    ('/**\n * Text.\n *\n *     std::fs::read("x");\n */\npub fn f() {}', HOST),
    ('#[doc = "Text.\\n\\n    std::fs::read(\\"x\\");"]\npub fn f() {}', HOST),
    ('const S: &str = include_str!("x");', INCLUDE),
    ("use core::include_str as i;", INCLUDE),
    ('#[cfg_attr(test, path = "x.rs")]\nmod x;', INCLUDE),
    ('/// ```\n/// #[path = "../x.rs"] mod external;\n/// ```\npub fn f() {}', INCLUDE),
    ('/// ```\n/// # #[path = "../x.rs"]\n/// # mod external;\n/// ```\npub fn f() {}', INCLUDE),
    ('/// ```\n/// #[cfg_attr(all(), path = "../x.rs")] mod external;\n/// ```\npub fn f() {}', INCLUDE),
    ('#[doc = "```\\n#[path = \\"../x.rs\\"] mod external;\\n```"]\npub fn f() {}', INCLUDE),
    ('/// ```text\n/// #[path = "../x.rs"] mod external;\n/// ```\npub fn f() {}', frozenset()),
    ('fn f() { expect_test::expect_file!["../../../docs/x.txt"].assert_eq(""); }', INCLUDE),
    ('fn f() { expect_file!["/tmp/x.txt"].assert_eq(""); }', INCLUDE),
    ('fn f() { expect_file![r"..\\..\\..\\x.txt"].assert_eq(""); }', INCLUDE),
    ("fn f() { expect_file![P].assert_eq(\"\"); }", INCLUDE),
    ("use expect_test::expect_file as e;", INCLUDE),
    ('/// ```\n/// expect_test::expect_file!["x.txt"];\n/// ```\npub fn f() {}', INCLUDE),
    ('fn f() { expect_file!["../tests/x.txt"].assert_eq(""); }', frozenset()),
    ("#[cfg(test)]\nmod t {}\n#[cfg(not(test))]\nfn f() {}\n#[cfg_attr(test, derive(Debug))]\nstruct S;", frozenset()),
    ('#[cfg(feature = "x")]\nfn f() {}', CFG),
    ("#[cfg_attr(test, cfg(any()))]\nfn f() {}", CFG),
    ("fn f() -> bool { cfg!(debug_assertions) }", CFG),
)


def scan_cases() -> list[Violation]:
    out = []
    for text, want in SCAN_CASES:
        src = parse_source("crates/ng-case/src/case.rs", text)
        got = {
            code
            for code, hits in (
                ("seam.host", host_hits(src)),
                ("seam.include", include_hits(src, "crates/ng-case")),
                ("seam.cfg", cfg_hits(src)),
            )
            if hits
        }
        if got != want:
            out.append(
                Violation(
                    "scan",
                    repr(text[:50]),
                    f"found {sorted(got) or 'nothing'}, the rules say {sorted(want) or 'nothing'}",
                )
            )
    return out


PROBE_LOCK = (
    "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n"
)


def write_probe(name: str, source: str) -> pathlib.Path:
    """A throwaway crate under target/gate/, its own workspace, beneath the root so clippy finds clippy.toml."""
    d = ROOT / "target/gate" / name
    (d / "src").mkdir(parents=True, exist_ok=True)
    (d / "Cargo.toml").write_text(
        f'[package]\nname = "gate-{name}"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[workspace]\n',
        encoding="utf-8",
    )
    (d / "Cargo.lock").write_text(
        PROBE_LOCK + f'\n[[package]]\nname = "gate-{name}"\nversion = "0.0.0"\n', encoding="utf-8"
    )
    (d / "src/lib.rs").write_text(source, encoding="utf-8")
    return d


# The stable f64 and f32 methods clippy.toml leaves allowed: predicates, bit and byte conversions, and
# IEEE 754 operations every host rounds the same way. A stable method on neither list fails the bans probe,
# so a toolchain that stabilizes another libm method (erf and gamma are unstable on 1.98.1) cannot slip in.
EXACT_FLOAT_METHODS = frozenset(
    {
        "abs",
        "abs_sub",
        "ceil",
        "clamp",
        "classify",
        "copysign",
        "div_euclid",
        "floor",
        "fract",
        "from_be_bytes",
        "from_bits",
        "from_le_bytes",
        "from_ne_bytes",
        "is_finite",
        "is_infinite",
        "is_nan",
        "is_normal",
        "is_sign_negative",
        "is_sign_positive",
        "is_subnormal",
        "midpoint",
        "next_down",
        "next_up",
        "recip",
        "rem_euclid",
        "round",
        "round_ties_even",
        "signum",
        "sqrt",
        "to_be_bytes",
        "to_bits",
        "to_degrees",
        "to_int_unchecked",
        "to_le_bytes",
        "to_ne_bytes",
        "to_radians",
        "total_cmp",
        "trunc",
    }
)


def _stable_methods(toks: list[Token], ty: str) -> set[str]:
    names, i = set(), 0
    while i < len(toks):
        if not (toks[i].value == "impl" and i + 2 < len(toks) and toks[i + 1].value == ty and _punct(toks, i + 2, "{")):
            i += 1
            continue
        depth, k, attrs = 1, i + 3, []
        while k < len(toks) and depth:
            t = toks[k]
            if t.kind == "punct" and t.value in ("(", "[", "{"):
                depth += 1
            elif t.kind == "punct" and t.value in (")", "]", "}"):
                depth -= 1
            elif depth == 1 and _punct(toks, k, "#") and _punct(toks, k + 1, "["):
                attrs.append(toks[k + 2].value)
            elif depth == 1 and t.value == ";":
                attrs = []
            elif depth == 1 and t.value == "fn":
                j = k - 1
                while toks[j].value in ("const", "unsafe"):
                    j -= 1
                if toks[j].value == "pub" and "stable" in attrs:
                    names.add(toks[k + 1].value)
                attrs = []
            k += 1
        i = k
    return names


def stable_float_methods() -> dict[str, frozenset[str]]:
    """Every stable inherent method of f64 and f32, read from the pinned toolchain's own library source."""
    sysroot = subprocess.run(["rustc", "--print", "sysroot"], capture_output=True, text=True).stdout.strip()
    lib = pathlib.Path(sysroot) / "lib/rustlib/src/rust/library"
    out = {}
    for ty in ("f64", "f32"):
        names: set[str] = set()
        for crate in ("core", "std"):
            path = lib / crate / "src/num" / f"{ty}.rs"
            if not path.is_file():
                raise GateError(f"{path} is missing: rust-src is not installed; run ensure-prereqs.sh")
            names |= _stable_methods(lex(_read(path))[0], ty)
        out[ty] = frozenset(names)
    return out


def float_gaps(methods: Mapping[str, frozenset[str]], banned: set[str]) -> list[str]:
    return sorted(
        f"{ty}::{m}"
        for ty, names in methods.items()
        for m in names
        if f"{ty}::{m}" not in banned and m not in EXACT_FLOAT_METHODS
    )


def bans() -> int:
    conf = _toml(ROOT / "clippy.toml")
    entries = {}
    lines = ["#![allow(unused_imports)]"]
    for kind, form in (
        ("disallowed-types", "use {} as _T{};"),
        ("disallowed-methods", "fn _m{1}() {{ let _ = {0}; }}"),
    ):
        for i, e in enumerate(conf.get(kind, [])):
            path = e["path"] if isinstance(e, dict) else e
            lines.append(form.format(path, i))
            entries[len(lines)] = path
    probe = write_probe("bans-probe", "\n".join(lines) + "\n")
    r = subprocess.run(
        [
            "cargo",
            "clippy",
            "--locked",
            "--offline",
            "--message-format=json",
            "--",
            "-F",
            "clippy::disallowed_methods",
            "-F",
            "clippy::disallowed_types",
        ],
        cwd=probe,
        capture_output=True,
        text=True,
    )
    named: dict[int, set[str]] = {}
    errors = []
    for out in r.stdout.splitlines():
        msg = json.loads(out).get("message") if out.startswith("{") else None
        if not msg:
            continue
        code = (msg.get("code") or {}).get("code")
        if code in ("clippy::disallowed_methods", "clippy::disallowed_types"):
            for span in (sp for sp in msg["spans"] if sp["is_primary"]):
                named.setdefault(span["line_start"], set()).update(re.findall(r"`([^`]+)`", msg["message"]))
        elif msg["level"] == "error":
            errors.append(Violation("bans", "probe", (msg.get("rendered") or msg["message"]).strip().splitlines()[0]))
    unnamed = [e for line, e in entries.items() if e not in named.get(line, set())]
    if r.returncode == 0:
        found = [Violation("bans", "probe", "clippy accepted it: a lint cap or a lowered level is in effect")]
    elif errors:
        found = errors[:5] + [
            Violation("bans", "probe", "it did not compile, so clippy checked no entry; fix the entry named")
        ]
    else:
        found = [Violation("bans", e, "clippy did not reject its probe line") for e in unnamed]
    methods = stable_float_methods()
    banned = {e["path"] if isinstance(e, dict) else e for e in conf.get("disallowed-methods", [])}
    gaps = float_gaps(methods, banned)
    found += [
        Violation(
            "bans",
            m,
            "a stable method clippy.toml does not ban and EXACT_FLOAT_METHODS does not name; ban it, "
            "or name it there if IEEE 754 makes its result the same on every host",
        )
        for m in gaps
    ]
    found += [
        Violation("bans", m, "EXACT_FLOAT_METHODS names no stable f64 or f32 method; drop it")
        for m in sorted(EXACT_FLOAT_METHODS - methods["f64"] - methods["f32"])
    ]
    found += [
        Violation("bans", m, "dropping its ban leaves no gap the check reports")
        for m in sorted(banned)
        if m.split("::")[-1] in methods.get(m.split("::")[0], ()) and m not in float_gaps(methods, banned - {m})
    ]
    for v in found:
        print(v)
    rejected = len(entries) - len(unnamed)
    print(
        f"10-rust: bans probe: {len(entries)} clippy.toml entries, {rejected} rejected on their own line; "
        f"{sum(map(len, methods.values()))} stable f64 and f32 methods, "
        + (f"{len(gaps)} neither banned nor exact" if gaps else "each banned or exact")
    )
    return 1 if found else 0


RUSTDOC_FORMAT = 60
F64_PROBE = """\
pub struct Pressure(f64);

pub fn leak_param(_x: f64) {}

pub fn leak_return() -> f32 {
    0.0
}

pub struct LeakField {
    pub value: f64,
}

pub struct LeakTuple(pub f64);

pub enum LeakVariant {
    Raw(f64),
    Named { value: f64 },
}

pub use core::primitive::f64 as LeakReexport;

pub use core::primitive::f32;

pub use core::primitive as leak_primitive;

pub trait LeakTrait {
    fn leak_method(&self) -> f64;
}

pub struct LeakFrom(f64);

impl From<f64> for LeakFrom {
    fn from(x: f64) -> Self {
        LeakFrom(x)
    }
}

pub const LEAK_CONST: f64 = 0.0;

pub type LeakAlias = f64;

impl Pressure {
    pub fn leak_inherent(&self) -> f64 {
        self.0
    }

    pub fn ok_typed(&self) -> &Pressure {
        self
    }

    fn ok_private(&self) -> f64 {
        self.0
    }
}

pub struct OkPrivateField {
    value: f64,
}

pub(crate) fn ok_crate_visible(x: f64) -> f64 {
    x
}

mod ok_private_module {
    pub fn ok_unreachable(x: f64) -> f64 {
        x
    }
}

#[doc(hidden)]
pub fn leak_hidden(_x: f64) {}

#[doc(hidden)]
pub mod hidden {
    pub fn leak_in_hidden_module(_x: f64) {}
}

pub trait LeakBlanket {
    type Unit;

    fn number(&self) -> Self::Unit;
}

impl<T> LeakBlanket for T {
    type Unit = f64;

    fn number(&self) -> f64 {
        0.0
    }
}
"""
F64_PROBE_LEAKS = frozenset(
    {
        "leak_param",
        "leak_return",
        "LeakField.value",
        "LeakTuple.0",
        "LeakVariant::Raw.0",
        "LeakVariant::Named.value",
        "LeakTrait::leak_method",
        "<LeakFrom as From>::from",
        "LEAK_CONST",
        "LeakAlias",
        "LeakReexport",
        "f32",
        "leak_primitive",
        "Pressure::leak_inherent",
        "leak_hidden",
        "hidden::leak_in_hidden_module",
        "<T as LeakBlanket>::Unit",
        "<T as LeakBlanket>::number",
    }
)


def _has_float(x: object) -> bool:
    if isinstance(x, dict):
        return x.get("primitive") in ("f64", "f32") or any(_has_float(v) for v in x.values())
    return isinstance(x, list) and any(_has_float(v) for v in x)


def _reexports_float(it: dict, paths: dict) -> bool:
    """A `use` of a primitive has no item to resolve, so its source path names it; `core::primitive` hands
    out every float under any alias."""
    if "use" not in it["inner"]:
        return False
    use = it["inner"]["use"]
    target = paths.get(str(use["id"])) if use["id"] is not None else None
    if target and target["kind"] == "module" and target["path"][1:] == ["primitive"]:
        return True
    return use["source"].split("::")[-1] in ("f64", "f32")


def float_items(doc: dict) -> set[str]:
    index, paths = doc["index"], doc["paths"]

    def path_of(i: object) -> str | None:
        p = paths.get(str(i))
        return "::".join(p["path"][1:]) if p and p["crate_id"] == 0 else None

    def type_name(t: dict) -> str:
        if "resolved_path" in t:
            return path_of(t["resolved_path"]["id"]) or t["resolved_path"]["path"].split("::")[-1]
        if "primitive" in t:
            return t["primitive"]
        if "borrowed_ref" in t:
            return "&" + type_name(t["borrowed_ref"]["type"])
        if "slice" in t:
            return f"[{type_name(t['slice'])}]"
        return t.get("generic", "_")

    parent: dict[str, tuple[str, str]] = {}
    for i, it in index.items():
        ((kind, body),) = it["inner"].items()
        shape = body.get("kind") if isinstance(body, dict) else None
        fields = []
        if isinstance(shape, dict):
            fields = (
                shape.get("tuple")
                or shape.get("plain", {}).get("fields")
                or shape.get("struct", {}).get("fields")
                or []
            )
        if kind == "union":
            fields = body["fields"]
        parent |= {str(f): ("field", i) for f in fields if f is not None}
        if kind in ("trait", "impl"):
            original = kind == "trait" or (not body["is_synthetic"] and body["blanket_impl"] is None)
            parent |= {str(m): (kind, i) for m in body["items"] if original or str(m) not in parent}

    def name(i: str) -> str:
        it = index[i]
        if i not in parent:
            return path_of(i) or it["name"] or ("use" in it["inner"] and it["inner"]["use"]["name"]) or f"item {i}"
        how, p = parent[i]
        if how == "field":
            return f"{name(p)}.{it['name']}"
        if how == "trait":
            return f"{name(p)}::{it['name']}"
        imp = index[p]["inner"]["impl"]
        if imp["trait"] is None:
            return f"{type_name(imp['for'])}::{it['name']}"
        return f"<{type_name(imp['for'])} as {imp['trait']['path'].split('::')[-1]}>::{it['name']}"

    def in_api_impl(i: str) -> bool:
        if i not in parent or parent[i][0] != "impl":
            return True
        imp = index[parent[i][1]]["inner"]["impl"]
        return not imp["is_synthetic"] and imp["blanket_impl"] is None

    return {
        name(i)
        for i, it in index.items()
        if it["crate_id"] == 0
        and "impl" not in it["inner"]
        and (_has_float(it["inner"]) or _reexports_float(it, paths))
        and in_api_impl(i)
    }


def rustdoc(cwd: pathlib.Path, crate: str | None) -> dict:
    target = (ROOT / (os.environ.get("CARGO_TARGET_DIR") or "target")).resolve() / "gate/rustdoc"
    package = ["-p", crate] if crate else []
    r = subprocess.run(
        [
            "cargo",
            "rustdoc",
            *package,
            "--lib",
            "--locked",
            "--target-dir",
            str(target),
            "--",
            "-Z",
            "unstable-options",
            "--document-hidden-items",
            "--output-format",
            "json",
        ],
        cwd=cwd,
        env={**os.environ, "RUSTC_BOOTSTRAP": "1"},
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        raise GateError(f"cargo rustdoc {crate or cwd.name}: {r.stderr.strip()}")
    lib = crate or tomllib.loads(_read(cwd / "Cargo.toml"))["package"]["name"]
    doc = json.loads(_read(target / "doc" / f"{lib.replace('-', '_')}.json"))
    if doc.get("format_version") != RUSTDOC_FORMAT:
        rustc = subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip()
        raise GateError(
            f"rustdoc JSON format_version {doc.get('format_version')} under {rustc}; the f64 scan "
            f"reads {RUSTDOC_FORMAT}, so update it with the toolchain"
        )
    return doc


def f64() -> int:
    ws = load(ROOT)
    crates = os.environ["NG_GATE_CRATES"].split() if "NG_GATE_CRATES" in os.environ else sorted(ws.members)
    found, allowed = [], 0
    for crate in crates:
        items = float_items(rustdoc(ROOT, crate))
        row = ws.registry.row(crate)
        allow = {a.item for a in row.f64_allow} if row else set()
        allowed += len(items & allow)
        found += [
            Violation(
                "f64",
                f"{item} in {crate}",
                "a bare f64 or f32 crosses the crate boundary; use a dimensioned "
                "type, or allow the item in the crate's [[crate]] row with a reason",
            )
            for item in sorted(items - allow)
        ]
        found += [
            Violation("f64", f"{item} in {crate}", "the f64_allow entry matches no public item; drop it")
            for item in sorted(allow - items)
        ]
    summary = f"15-f64: {len(crates)} crates, {allowed} allowed items with a bare f64 or f32"
    if os.environ.get("NG_GATE_MODE") != "changed":
        flagged = float_items(rustdoc(write_probe("f64-probe", F64_PROBE), None))
        found += [
            Violation("f64", f"{item} in the probe", "flagged, but the probe declares it private or typed")
            for item in sorted(flagged - F64_PROBE_LEAKS)
        ]
        found += [
            Violation("f64", f"{item} in the probe", "a public bare f64 the scan no longer flags")
            for item in sorted(F64_PROBE_LEAKS - flagged)
        ]
        summary += (
            f"; probe: {len(flagged & F64_PROBE_LEAKS)} of {len(F64_PROBE_LEAKS)} leaks flagged, "
            f"{len(flagged - F64_PROBE_LEAKS)} other items"
        )
    for v in found:
        print(v)
    print(summary)
    return 1 if found else 0


DOCTEST_PROBE = """\
//! ```
//! let _ = 1;
//! ```

/// ```edition2021
/// let _ = 2;
/// ```
pub fn fenced() {}

/// Text.
///
///     let _ = 3;
pub fn indented() {}

#[doc = concat!("```\\n", "let _ = 4;\\n", "```")]
pub fn concatenated() {}

macro_rules! documented {
    ($d:literal) => {
        #[doc = $d]
        pub fn generated() {}
    };
}

documented!("```\\nlet _ = 5;\\n```");
"""


def listed_doctests(cwd: pathlib.Path, crates: list[str]) -> list[tuple[str, str, int]]:
    """rustdoc's own list as (crate, file, line), so the host scan's reading of the docs is checked, not trusted."""
    r = subprocess.run(
        ["cargo", "test", "--doc", "--locked", *(a for c in crates for a in ("-p", c)), "--", "--list"],
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if r.returncode != 0:
        raise GateError(f"cargo test --doc -- --list: {r.stdout.strip()}")
    crate, out = "", []
    for line in r.stdout.splitlines():
        if m := re.fullmatch(r"\s*Doc-tests (\S+)", line):
            crate = m.group(1).replace("_", "-")
        elif m := re.fullmatch(r"(\S+) - .*\(line (\d+)\): test", line):
            out.append((crate, m.group(1), int(m.group(2))))
    return out


def unread(src: Source | None, line: int) -> bool:
    """rustdoc reports a block comment's fence one line early."""
    return src is None or not {line, line + 1} & set(src.blocks)


def doctests() -> int:
    ws = load(ROOT)
    crates = os.environ["NG_GATE_CRATES"].split() if "NG_GATE_CRATES" in os.environ else sorted(ws.members)
    listed = listed_doctests(ROOT, crates) if crates else []
    found = []
    for crate, file, line in listed:
        m = ws.members.get(crate)
        src = next((s for s in m.sources if s.path in (f"{m.dir}/{file}", file)), None) if m else None
        if unread(src, line):
            found.append(
                Violation(
                    "doctest",
                    f"{src.path if src else file}:{line}",
                    "rustdoc runs a doctest here that the host scan cannot read, such as one a macro or concat! "
                    "builds; write it as a /// code block",
                )
            )
    summary = f"10-rust: doctests: {len(listed)} listed by rustdoc, {len(listed) - len(found)} read by the host scan"
    if os.environ.get("NG_GATE_MODE") != "changed":
        src = parse_source("src/lib.rs", DOCTEST_PROBE)
        got = {
            line for _, _, line in listed_doctests(write_probe("doctest-probe", DOCTEST_PROBE), []) if unread(src, line)
        }
        want = {
            n
            for n, text in enumerate(DOCTEST_PROBE.splitlines(), 1)
            if text.startswith(("#[doc = concat!", "documented!("))
        }
        if got != want:
            found.append(
                Violation(
                    "doctest",
                    "the probe",
                    f"unread doctests at lines {sorted(got)}; the probe has them at {sorted(want)}",
                )
            )
        summary += f"; probe: {len(want)} unreadable doctests flagged"
    for v in found:
        print(v)
    print(summary)
    return 1 if found else 0


def _state_path() -> pathlib.Path:
    return pathlib.Path.home() / ".local/state/nightingale/nightly.json"


def nightly_state() -> dict | None:
    try:
        state = json.loads(_state_path().read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None
    except (OSError, ValueError) as e:
        raise GateError(f"{_state_path()}: {e}") from None
    if not isinstance(state, dict):
        raise GateError(f"{_state_path()}: not a JSON object")
    return state


def nightly_record(status: int, log: pathlib.Path) -> int:
    sha = git(ROOT, "rev-parse", "HEAD")
    try:
        last_green = (nightly_state() or {}).get("last_green")
    except GateError:
        last_green = None
    state = {
        "sha": sha,
        "result": "passed" if status == 0 else "failed",
        "finished_utc": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "log": str(log.resolve()),
        "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
        "last_green": sha if status == 0 else last_green,
    }
    path = _state_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=".nightly-", suffix=".json")
    with os.fdopen(fd, "w", encoding="utf-8") as out:
        json.dump(state, out, indent=2)
        out.write("\n")
    os.replace(tmp, path)
    print(f"nightly: {state['result']} on {sha}, recorded in {path}")
    return 0


def changed_run_warning() -> None:
    try:
        state = nightly_state()
    except GateError as e:
        print(f"gate: warning: the nightly state is unreadable ({e})", file=sys.stderr)
        return
    if state and state.get("result") == "failed":
        print(
            f"gate: warning: the last nightly full run failed on {state.get('sha')} (log {state.get('log')}); "
            "tools/verify --full fails until main is fixed",
            file=sys.stderr,
        )


def seam() -> int:
    ws = load(ROOT)
    found = check(ws, "registration") + check(ws, "seam")
    for v in found:
        print(v)
    probes, bites = run_probes(ws, tree_passes=not found)
    cases = plan_cases(ws) + scan_cases()
    for v in probes + cases:
        print(v)
    print(
        f"05-seam: {len(ws.members)} members against the crate table; {len(RULES)} rules, {bites} probes bite; "
        f"{len(PLAN_CASES)} plan cases; {len(SCAN_CASES)} scan cases"
    )
    return 1 if found or probes or cases else 0


def plan_command(mode: Mode, base: str | None) -> int:
    print(plan(load(ROOT), mode, base).lines())
    if mode == "changed":
        changed_run_warning()
    return 0


def main(argv: list[str]) -> int:
    try:
        match argv:
            case ["plan", "merge" | "changed" | "full" as mode]:
                return plan_command(mode, None)
            case ["plan", "merge" | "changed" as mode, "--base", base]:
                return plan_command(mode, base)
            case ["seam"]:
                return seam()
            case ["bans"]:
                return bans()
            case ["f64"]:
                return f64()
            case ["doctests"]:
                return doctests()
            case ["nightly-record", status, log] if status.isdigit():
                return nightly_record(int(status), pathlib.Path(log))
            case ["nightly-passed", sha]:
                state = nightly_state() or {}
                return 0 if state.get("sha") == sha and state.get("result") == "passed" else 1
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


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
