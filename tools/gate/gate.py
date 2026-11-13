"""The gate's policy: which steps and crates a run covers, and the workspace rules 05-seam holds.

Python 3.11 stdlib only. Run as `python3 -I tools/gate/gate.py <command>`; it finds the repo from its own path.

    plan <merge|changed|full> [--base REV]   the run's plan as key=value lines
    seam                                     the workspace rules, every rule's probe, the plan, scan and comment cases
    mutants [first|rest]                     40-mutants: first checks that every row applies, then sweeps the rows the
                                             change adds or edits or whose path it changes; rest sweeps the run's
                                             other rows, skipping the baselines 10-rust passed; with no part, both
                                             in turn. A sweep stops at its first row that is not caught
    tested --reset | -- ARGS                 10-rust's record of the `cargo test` calls it passed on a clean HEAD
    bans                                     clippy rejects a probe built from clippy.toml, entry by entry
    f64                                      no bare f64 or f32 in a public API outside its row's allow list
    nightly-record <exit> <log>              record a nightly full run (tools/gate/nightly.sh)
    nightly-passed <sha>                     exit 0 when the nightly state records sha as passed
    doctests                                 every doctest rustdoc lists is one the host scan read
    codes [DIR]                              every compile_fail doctest fails with the error code it declares,
                                             in the planned crates or in the standalone crate at DIR
    comments-only [--base REV]               only comments changed against REV, by default the merge-base with
                                             origin/main: each changed path is a Rust file whose code tokens,
                                             doctest tokens and doctest fences are unchanged

Exit codes: 0 pass; 1 violations, one per line as `<code>: <subject>: <what to do>`; 2 an input the gate
reads is missing or malformed.
"""

from __future__ import annotations

import contextlib
import dataclasses
import datetime
import fcntl
import functools
import hashlib
import importlib.util
import io
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
import time
import tomllib
from collections.abc import Callable, Iterable, Mapping, Sequence, Set
from typing import Literal

Mode = Literal["merge", "changed", "full"]
Scope = Literal["docs-only", "affected", "everything"]
Phase = Literal["registration", "seam"]

ROOT = pathlib.Path(__file__).resolve().parents[2]
PATHS = "tools/gate/paths.toml"
MUST_FAIL = "tools/gate/must-fail.toml"
MUTANTS = "tools/gate/mutants.toml"
MUTATE = "tools/mutate.py"
TESTED = "target/gate/tested.jsonl"
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
class SealSkip:
    item: str
    field: str
    reason: str


@dataclasses.dataclass(frozen=True)
class CrateRow:
    name: str
    smoke: tuple[str, ...] = ()
    dev_external: Mapping[str, str] = dataclasses.field(default_factory=dict)
    f64_allow: tuple[F64Allow, ...] = ()
    seal_skip: tuple[SealSkip, ...] = ()


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
        _table(c, where, {"name"}, {"smoke", "dev_external", "f64_allow", "seal_skip"})
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
        skips = []
        for j, a in enumerate(c.get("seal_skip", [])):
            a = _table(a, f"{where}: seal_skip {j + 1}", {"item", "field", "reason"})
            if not all(isinstance(a[k], str) and a[k].strip() for k in ("item", "field", "reason")):
                raise GateError(f"{where}: seal_skip {j + 1}: item, field and reason must be non-empty")
            skips.append(SealSkip(a["item"], a["field"], a["reason"]))
        crates.append(
            CrateRow(
                c["name"], _strings(c.get("smoke", []), f"{where}: smoke"), dict(dev), tuple(allows), tuple(skips)
            )
        )
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
    written: str = ""
    joint: bool = False


@dataclasses.dataclass(frozen=True)
class Source:
    path: str
    code: tuple[Token, ...]
    doctests: tuple[Token, ...]
    blocks: tuple[int, ...]
    docblocks: tuple[tuple[int, int, str, str], ...]


class LexError(Exception):
    pass


RAW_STRING = re.compile(r'[bc]?r(#*)"')
QUOTED = re.compile(r'[bc]?"')


OPERATOR_PAIRS = frozenset(
    {"::", "->", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "%=", "^=", "&=", "|=", "<<", ">>",
     "..", ".="}
)


def _joins(text: str, i: int) -> bool:
    return text[i : i + 2] in OPERATOR_PAIRS


def _suffixed(text: str, j: int) -> bool:
    """rustc reads an identifier touching a literal as its suffix, so `"a"tag` and `"a" tag` differ."""
    return text[j : j + 1].isidentifier()


def _float_dot(text: str, i: int, j: int) -> bool:
    """rustc reads `1.` as one float where `1 .` is two tokens."""
    after = text[j + 1 : j + 2]
    decimal = text[i:j].replace("_", "").isdigit()
    return decimal and text[j : j + 1] == "." and not (after == "." or after.isidentifier())


def _block_doc(text: str) -> str:
    """rustc trims a block doc's `*` margin only when every line has one in the same column, and then drops a `*`
    that stands alone or comes before a space or another `*`, so a line such as `*b;` stays code."""
    lines = text.split("\n")
    k, end = (0 if lines[0].lstrip().startswith("*") else 1), len(lines)
    while k < end and not lines[k].strip():
        k += 1
    while end > k and not lines[end - 1].strip():
        end -= 1
    margins = [m.group(1) for ln in lines[k:end] if (m := re.match(r"([ \t]*)\*", ln))]
    if k == end or len(margins) != end - k or len(set(margins)) != 1:
        return text
    prefix, out = margins[0], []
    for ln in lines:
        if ln.startswith(prefix):
            ln = ln[len(prefix) :]
            ln = ln[1:] if ln == "*" or ln.startswith(("* ", "**")) else ln
        out.append(ln)
    return "\n".join(out)


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
                docs.append((line, _block_doc(body[1:])))
            line += text.count("\n", i, j)
            i = j
        elif m := RAW_STRING.match(text, i):
            close = '"' + m.group(1)
            j = text.find(close, m.end())
            if j < 0:
                raise LexError(f"line {line}: unterminated raw string")
            end = j + len(close)
            tokens.append(Token("literal", text[m.end() : j], start_line, text[i:end], _suffixed(text, end)))
            line += text.count("\n", i, j)
            i = end
        elif m := QUOTED.match(text, i):
            j = _skip_quoted(text, m.end(), '"')
            tokens.append(Token("literal", text[m.end() : j - 1], start_line, text[i:j], _suffixed(text, j)))
            line += text.count("\n", i, j)
            i = j
        elif text.startswith("b'", i) or (c == "'" and (text.startswith("\\", i + 1) or text.startswith("'", i + 2))):
            q = i + 1 if c == "b" else i
            j = _skip_quoted(text, q + 1, "'")
            tokens.append(Token("literal", text[q + 1 : j - 1], start_line, text[i:j], _suffixed(text, j)))
            i = j
        elif c == "'":
            j = i + 3 if text.startswith("r#", i + 1) and text[i + 3 : i + 4].isidentifier() else i + 1
            while j < n and (text[j] == "_" or text[j].isalnum()):
                j += 1
            tokens.append(Token("lifetime", text[i:j], start_line))
            i = j
        elif text.startswith("r#", i) and i + 2 < n and (text[i + 2] == "_" or text[i + 2].isalpha()):
            j = i + 2
            while j < n and (text[j] == "_" or text[j].isalnum()):
                j += 1
            tokens.append(Token("ident", text[i + 2 : j], start_line, text[i:j]))
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
            tokens.append(Token("literal", text[i:j], start_line, joint=_float_dot(text, i, j)))
            i = j
        elif text.startswith("::", i):
            tokens.append(Token("punct", "::", start_line, joint=_joins(text, i + 1)))
            i += 2
        else:
            tokens.append(Token("punct", c, start_line, joint=_joins(text, i)))
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


def rust_blocks(docs: list[tuple[int, str]]) -> list[tuple[int, int, str, str]]:
    """The code blocks rustdoc runs: fenced blocks it reads as Rust, and lines indented four past the docs'
    own indent after a blank line. That second rule also takes a list item's indented paragraph, which
    rustdoc does not run: an over-reach, never a miss."""
    blocks = []
    for group in _doc_groups(docs):
        base = min((len(t) - len(t.lstrip()) for _, t in group if t.strip()), default=0)
        fence, rust, start, body, blank, opening = None, False, 0, [], True, ""
        for n, ln in group:
            indented = bool(ln.strip()) and len(ln) - len(ln.lstrip()) >= base + 4
            if fence is not None:
                if ln.strip().startswith(fence) and not ln.strip().strip(fence[0]):
                    if rust:
                        blocks.append((start - 1, start, "\n".join(body), f"{opening}\n{ln.strip()}"))
                    fence, body = None, []
                else:
                    body.append(ln)
            elif body and (indented or not ln.strip()):
                body.append(ln)
            else:
                if body:
                    blocks.append((start, start, "\n".join(body), ""))
                    body = []
                if indented and blank:
                    start, body = n, [ln]
                elif m := FENCE.match(ln):
                    fence, start, rust, opening = m.group(1), n + 1, _is_rust_fence(m.group(2)), ln.strip()
            blank = not ln.strip()
        if body and (fence is None or rust):
            blocks.append((start - 1 if fence else start, start, "\n".join(body), opening if fence else ""))
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
    for _, start, block, _ in blocks:
        try:
            doctests += lex(block, start)[0]
        except LexError:
            doctests += [Token("ident", w, start) for w in re.findall(r"[A-Za-z_]\w*", block)]
    return Source(path, tuple(code), tuple(doctests), tuple(b[0] for b in blocks), tuple(blocks))


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
    test: bool
    doctest: bool


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
            Target(t["name"], frozenset(t["kind"]), pathlib.Path(t["src_path"]).resolve(), t["test"], t["doctest"])
            for t in p["targets"]
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


def _closing(code: tuple[Token, ...], opening: int) -> int:
    depth = 0
    for k in range(opening, len(code)):
        if code[k].value == "{":
            depth += 1
        elif code[k].value == "}":
            depth -= 1
            if depth == 0:
                return k
    return len(code) - 1


def _impl_target(head: tuple[Token, ...]) -> tuple[str, bool]:
    values = [t.value for t in head]
    start = 0
    if values[:1] == ["<"]:
        depth = 0
        for start, v in enumerate(values):
            depth += (v == "<") - (v == ">")
            if depth == 0:
                break
        start += 1
    rest = values[start:]
    if "where" in rest:
        rest = rest[: rest.index("where")]
    seal = False
    if "for" in rest:
        at = rest.index("for")
        seal = "Seal" in rest[:at]
        rest = rest[at + 1 :]
    path = rest[: rest.index("<")] if "<" in rest else rest
    return next((v for v in reversed(path) if v[:1].isalpha() or v[:1] == "_"), "?"), seal


def _opens_identity(body: tuple[Token, ...]) -> bool:
    for k in range(len(body) - 3):
        if body[k + 1].value != "::":
            continue
        if body[k].value == "Canonical" and body[k + 2].value == "new":
            return True
        constructor = body[k + 2].value in ("of_canonical", "for_record")
        if body[k].value == "Digest" and constructor and body[k + 3].value == "(":
            if k + 4 < len(body) and body[k + 4].kind != "literal":
                return True
    return False


def _signature_end(code: tuple[Token, ...], start: int) -> int:
    depth = 0
    for k in range(start, len(code)):
        v = code[k].value
        if v in ("(", "["):
            depth += 1
        elif v in (")", "]"):
            depth -= 1
        elif depth == 0 and v in ("{", ";"):
            return k
    return len(code)


def seal_scopes(src: Source) -> list[tuple[str, tuple[Token, ...]]]:
    code, stem = src.code, pathlib.PurePosixPath(src.path).stem
    items: list[tuple[int, int, int, str, bool, bool]] = []
    for i, t in enumerate(code):
        if t.kind != "ident":
            continue
        an_impl = t.value == "impl"
        named_fn = t.value == "fn" and i + 1 < len(code) and code[i + 1].kind == "ident"
        if not (an_impl or named_fn):
            continue
        j = _signature_end(code, i + 1)
        if j == len(code) or code[j].value == ";":
            continue
        if an_impl:
            items.append((i, j, _closing(code, j), *_impl_target(code[i + 1 : j]), True))
        else:
            items.append((i, j, _closing(code, j), code[i + 1].value, False, False))
    out = []
    for start, opening, close, name, _, is_impl in items:
        if is_impl:
            continue
        around = [item for item in items if item[1] < start <= item[2]]
        owner = max(around, key=lambda item: item[1], default=None)
        impl = owner if owner is not None and owner[5] else None
        inner = [item for item in items if opening < item[0] <= close]
        own = tuple(
            code[k] for k in range(start, close + 1) if not any(item[0] <= k <= item[2] for item in inner)
        )
        if (impl is not None and impl[4] and name == "seal") or _opens_identity(own):
            out.append(("::".join([stem, *([impl[3]] if impl is not None else []), name]), own))
    return out


def seal_holes(body: tuple[Token, ...]) -> list[tuple[int, str]]:
    out = []
    for k in range(len(body) - 2):
        a, b, c = body[k], body[k + 1], body[k + 2]
        if a.value == "." and b.value == "." and c.value == "}":
            out.append((a.line, ".."))
        elif a.kind == "ident" and b.value == ":" and c.value == "_" and k + 3 < len(body):
            if body[k + 3].value in (",", "}"):
                out.append((a.line, a.value))
    return out


@rule(
    "seam.seal",
    "seam",
    edit("crates/ng-contract/src/body.rs", r"^            judged,$", "            judged: _,"),
)
def seals_name_every_field(ws: Workspace) -> list[Violation]:
    """A destructure makes a new field E0027 until a seal names it, and `name: _` names it without sealing it."""
    code, out = "seam.seal", []
    for m in sorted(ws.members.values(), key=lambda m: m.name):
        row = ws.registry.row(m.name)
        listed = {(s.item, s.field) for s in row.seal_skip} if row else set()
        seen: set[tuple[str, str]] = set()
        for src in m.sources:
            if not src.path.startswith(f"{m.dir}/src/"):
                continue
            for item, body in seal_scopes(src):
                for line, hole in seal_holes(body):
                    if hole == "..":
                        out.append(
                            Violation(code, f"{src.path}:{line}", f"`..` leaves fields out of {item}; name each one")
                        )
                        continue
                    seen.add((item, hole))
                    if (item, hole) not in listed:
                        out.append(
                            Violation(
                                code,
                                f"{src.path}:{line}",
                                f"`{hole}: _` leaves a field out of {item}; seal it, move it out of what is sealed, "
                                f"or list it as a seal_skip of {m.name} in {PATHS} with its reason",
                            )
                        )
        out += [
            Violation(code, f"{item} {field} in {m.name}", "the seal_skip entry matches no field left out; drop it")
            for item, field in sorted(listed - seen)
        ]
    return out


MUST_FAIL_HEADER = "| Must-fail core or model | Rejected by |"


@dataclasses.dataclass(frozen=True)
class Doctest:
    path: str
    code: str
    text: str


@dataclasses.dataclass(frozen=True)
class MustFailRow:
    core: str
    mutants: tuple[str, ...]
    doctests: tuple[Doctest, ...]


def must_fail_rows(text: str) -> list[str]:
    lines = text.splitlines()
    heads = [i for i, line in enumerate(lines) if line.strip() == MUST_FAIL_HEADER]
    if len(heads) != 1:
        raise GateError(f"{CONTRACT}: expected one must-fail table headed {MUST_FAIL_HEADER!r}, found {len(heads)}")
    rows, i = [], heads[0] + 2
    while i < len(lines) and lines[i].startswith("|"):
        cells = [c.strip() for c in lines[i].strip().strip("|").split("|")]
        if len(cells) != 2 or not cells[0]:
            raise GateError(f"{CONTRACT}:{i + 1}: a must-fail row has two cells")
        rows.append(cells[0])
        i += 1
    return rows


def parse_must_fail(text: str) -> list[MustFailRow]:
    try:
        raw = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        raise GateError(f"{MUST_FAIL}: {e}") from None
    _table(raw, MUST_FAIL, {"row"})
    out = []
    for i, r in enumerate(raw["row"]):
        where = f"{MUST_FAIL}: row {i + 1}"
        _table(r, where, {"core"}, {"mutants", "doctests"})
        if not isinstance(r["core"], str) or not r["core"]:
            raise GateError(f"{where}: core is the row's first cell")
        doctests = []
        for j, d in enumerate(r.get("doctests", [])):
            d = _table(d, f"{where}: doctest {j + 1}", {"path", "code", "text"})
            if not all(isinstance(d[k], str) and d[k] for k in ("path", "code", "text")) or not re.fullmatch(
                r"E\d{4}", d["code"]
            ):
                raise GateError(f"{where}: doctest {j + 1}: a path, an error code such as E0599 and a line it holds")
            doctests.append(Doctest(d["path"], d["code"], d["text"]))
        out.append(MustFailRow(r["core"], _strings(r.get("mutants", []), f"{where}: mutants"), tuple(doctests)))
    return out


def mutant_tests(root: pathlib.Path) -> dict[str, str]:
    table = _toml(root / MUTANTS)
    return {m["name"]: m.get("test", table.get("test")) for m in table.get("mutant", [])}


def declares_test(src: Source, name: str) -> bool:
    code = src.code
    for k in range(len(code) - 1):
        if code[k].value != "fn" or code[k + 1].value != name:
            continue
        j = k - 1
        while j >= 0 and _punct(code, j, "]"):
            depth, i = 0, j
            while i >= 0:
                depth += _punct(code, i, "]") - _punct(code, i, "[")
                if depth == 0:
                    break
                i -= 1
            if [t.value for t in code[i + 1 : j]] == ["test"]:
                return True
            j = i - 2
    return False


def missing_test(ws: Workspace, test: str) -> str | None:
    args = test.split()
    valued = {"-p", "--package", "--test"}
    crates = [b for a, b in zip(args, args[1:]) if a in ("-p", "--package")]
    targets = [b for a, b in zip(args, args[1:]) if a == "--test"]
    names = [a for k, a in enumerate(args) if not a.startswith("-") and (k == 0 or args[k - 1] not in valued)]
    if len(crates) != 1 or len(targets) + ("--lib" in args) != 1 or len(names) != 1:
        return "name one crate with -p, one target with --test or --lib, and one test"
    m = ws.members.get(crates[0])
    if m is None:
        return f"{crates[0]} is no workspace member"
    module, _, name = names[0].rpartition("::")
    if targets:
        target = next((t for t in m.targets if t.name == targets[0] and "test" in t.kinds), None)
        files = [_rel(ws.root, target.src)] if target else []
    elif module:
        top = module.split("::")[0]
        files = [f"{m.dir}/src/{top}.rs", f"{m.dir}/src/{top}/mod.rs"]
    else:
        files = [f"{m.dir}/src/lib.rs"]
    if not any(declares_test(s, name) for s in m.sources if s.path in files):
        return f"no #[test] fn {name} in {' or '.join(files) or f'a test target {targets[0]} of {m.name}'}"
    return None


def missing_doctest(ws: Workspace, d: Doctest) -> str | None:
    src = next((s for m in ws.members.values() for s in m.sources if s.path == d.path), None)
    if src is None:
        return "names no Rust source of a workspace member"
    held = [
        line
        for line, _, block, fences in src.docblocks
        if {"compile_fail", d.code} <= set(_fence_words(fences.split("\n")[0].lstrip("`~"))) and d.text in block
    ]
    if len(held) != 1:
        return f"{len(held)} compile_fail,{d.code} doctests hold {d.text!r}; quote a line only one of them holds"
    return None


@rule(
    "seam.must-fail",
    "seam",
    edit(CONTRACT, r"^(\| An orphan pool \|)", r"| A core no test builds | Nothing |\n\1"),
)
def must_fail_rows_are_paired(ws: Workspace) -> list[Violation]:
    code = "seam.must-fail"
    rows = must_fail_rows(_read(ws.root / CONTRACT))
    entries = parse_must_fail(_read(ws.root / MUST_FAIL))
    mutants = mutant_tests(ws.root)
    named = [e.core for e in entries]
    out = []
    for row in dict.fromkeys(rows):
        if (n := named.count(row)) != 1 or rows.count(row) != 1:
            what = "no [[row]]" if n == 0 else f"{n} [[row]] entries" if n > 1 else "a second row of the table"
            out.append(Violation(code, row, f"{what} of {MUST_FAIL} names it; give it one, naming what rejects it"))
    for e in entries:
        if e.core not in rows:
            out.append(Violation(code, e.core, f"names no row of the must-fail table in {CONTRACT}; copy its first cell"))
        if not e.mutants and not e.doctests:
            out.append(Violation(code, e.core, "names no mutant and no compile_fail doctest"))
        out += [Violation(code, name, f"names no [[mutant]] of {MUTANTS}") for name in e.mutants if name not in mutants]
    return out


@rule(
    "seam.must-fail-test",
    "seam",
    edit("crates/ng-validate/tests/replay.rs", r"^fn a_core_that_reads_the_host_clock_fails_pace\(", "fn renamed("),
)
def must_fail_tests_exist(ws: Workspace) -> list[Violation]:
    code, out = "seam.must-fail-test", []
    mutants = mutant_tests(ws.root)
    for e in parse_must_fail(_read(ws.root / MUST_FAIL)):
        for name in e.mutants:
            if (test := mutants.get(name)) and (why := missing_test(ws, test)):
                out.append(Violation(code, f"{name}: {test}", why))
        out += [
            Violation(code, f"{d.path} {d.code}", why) for d in e.doctests if (why := missing_doctest(ws, d))
        ]
    return out


PROBE_COPY = (
    "Cargo.toml",
    "Cargo.lock",
    "clippy.toml",
    "rustfmt.toml",
    "rust-toolchain.toml",
    PATHS,
    CONTRACT,
    MUST_FAIL,
    MUTANTS,
)


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
    first: tuple[str, ...] = ()
    rest: tuple[str, ...] = ()

    def lines(self) -> str:
        return "\n".join(
            [
                f"mode={self.mode}",
                f"scope={self.scope}",
                f"base={self.base or '-'}",
                f"reason={self.reason}",
                f"crates={' '.join(self.crates)}",
                f"smoke={' '.join(self.smoke)}",
                f"first={' '.join(self.first)}",
                f"rest={' '.join(self.rest)}",
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


def split_tests(ws: Workspace, p: Plan, changes: Mapping[str, Modes]) -> Plan:
    """`--test NAME` runs NAME in every planned crate that has one."""
    if p.mode != "merge":
        return p
    members = [ws.members[c] for c in p.crates]
    tests = [t for m in members for t in m.targets if t.kinds == {"test"}]

    def edited(t: Target) -> bool:
        src = _rel(ws.root, t.src)
        home = posixpath.dirname(src) + "/" if posixpath.basename(src) == "main.rs" else None
        return any(path == src or (home is not None and path.startswith(home)) for path in changes)

    first = {t.name for t in tests if edited(t)}
    splittable = all(
        any(t.kinds == {"lib"} for t in m.targets)
        and all(t.test and (t.kinds == {"test"} or (t.kinds == {"lib"} and t.doctest)) for t in m.targets)
        for m in members
    )
    if not first or not splittable:
        return p
    return dataclasses.replace(p, first=tuple(sorted(first)), rest=tuple(sorted({t.name for t in tests} - first)))


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
    changes = changed_paths(ws.root, base)
    return split_tests(ws, plan_changes(ws, mode, base, changes), changes)


@dataclasses.dataclass(frozen=True)
class PlanCase:
    mode: Mode
    path: str
    scope: Scope
    crates: frozenset[str] | None = None
    reaches: frozenset[str] = frozenset()
    steps: frozenset[str] = frozenset()
    modes: Modes = (REGULAR, REGULAR)
    first: tuple[str, ...] = ()


RECORD_USERS = frozenset({"ng-record", "ng-observe", "ng-validate"})
PLAN_CASES = (
    PlanCase("merge", "docs/srs.md", "docs-only", frozenset(), steps=frozenset({"10-docs"})),
    PlanCase("merge", ".scratch/foundation/map.md", "docs-only", frozenset(), steps=frozenset({"30-data"})),
    PlanCase("merge", "data/closure.jsonl", "affected", frozenset(), steps=frozenset({"30-data"})),
    PlanCase("merge", "clinical-workspace/src/chart.ts", "affected", frozenset(), steps=frozenset({"50-workspace"})),
    PlanCase("merge", "AGENTS.md", "docs-only", frozenset(), steps=frozenset({"10-docs"})),
    PlanCase("merge", CONTRACT, "docs-only", frozenset(), steps=frozenset({"05-seam", "10-docs", "20-sketch"})),
    PlanCase(
        "merge", "docs/atlas/coupling-map.md", "docs-only", frozenset(), steps=frozenset({"10-docs", "20-sketch"})
    ),
    PlanCase("merge", "crates/ng-record/src/lib.rs", "affected", RECORD_USERS),
    PlanCase("changed", "crates/ng-record/src/lib.rs", "affected", frozenset({"ng-record"})),
    PlanCase("merge", "crates/ng-contract/src/lib.rs", "affected", reaches=RECORD_USERS | {"ng-contract"}),
    PlanCase(
        "merge", "crates/ng-validate/tests/window.rs", "affected", reaches=frozenset({"ng-validate"}), first=("window",)
    ),
    PlanCase("changed", "crates/ng-validate/tests/window.rs", "affected", frozenset({"ng-validate"})),
    PlanCase("merge", "crates/ng-validate/tests/replay_cores/mod.rs", "affected", reaches=frozenset({"ng-validate"})),
    PlanCase("merge", "crates/ng-contract/src/catalog.rs", "everything"),
    PlanCase("merge", "crates/ng-contract/src/system.rs", "everything"),
    PlanCase("merge", "Cargo.lock", "everything"),
    PlanCase("merge", "tools/verify", "everything"),
    PlanCase("merge", "notes.txt", "everything"),
    PlanCase("merge", "docs/srs.md", "everything", modes=(REGULAR, "100755")),
    PlanCase("merge", "AGENTS.md", "everything", modes=(REGULAR, "120000")),
)


def plan_cases(ws: Workspace) -> list[Violation]:
    out = []
    for c in PLAN_CASES:
        changes = {c.path: c.modes}
        p = split_tests(ws, plan_changes(ws, c.mode, "base", changes), changes)
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
        if p.first != c.first:
            got, want = " ".join(p.first) or "none", " ".join(c.first) or "none"
            out.append(Violation("plan", subject, f"first {got}, expected {want}"))
        tests = {t.name for n in p.crates for t in ws.members[n].targets if t.kinds == {"test"}}
        if p.rest != (tuple(sorted(tests - set(c.first))) if c.first else ()):
            want = "every other test target of the planned crates" if c.first else "none"
            out.append(Violation("plan", subject, f"rest {' '.join(p.rest) or 'none'}, expected {want}"))
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


SEAL_CASES: tuple[tuple[str, frozenset[str]], ...] = (
    (
        "impl Seal for T { fn seal(&self, out: &mut Canonical) { let Self { a, b: _ } = self; a.seal(out); } }",
        frozenset({"case::T::seal b"}),
    ),
    (
        'impl T { fn digest(&self) -> D { let Self { a, .. } = self; let o = Canonical::new("t"); o.digest() } }',
        frozenset({"case::T::digest .."}),
    ),
    (
        "impl<K: P> Q<K> where for<'a> K: R<'a> { fn hash(&self) -> H { let Self { kind: _, x } = self; "
        'Canonical::new("q"); } }',
        frozenset({"case::Q::hash kind"}),
    ),
    (
        "impl S { fn case(&self) -> D { let Self { a, b: _ } = self; Digest::of_canonical(&bytes) } }",
        frozenset({"case::S::case b"}),
    ),
    (
        "impl S { fn case(&self) -> D { let Self { a, b: _ } = self; Digest::for_record(&bytes, hash) } }",
        frozenset({"case::S::case b"}),
    ),
    ('fn tag(d: D) -> Digest { let D { key: _, v } = d; Digest::for_record(b"tag", hash) }', frozenset()),
    ('fn digest(&self) -> D { let o = Canonical::new("t"); let x = T { a, ..base }; o.digest() }', frozenset()),
    ("fn copy(d: D) -> S { let D { key: _, value } = d; S { value } }", frozenset()),
    (
        "struct F { read: fn(&R) -> bool }\nimpl Seal for G { fn seal(&self, out: &mut Canonical) { "
        "let Self { a: _ } = self; } }",
        frozenset({"case::G::seal a"}),
    ),
    (
        'fn digest(t: &T, salt: &[u8; 32]) -> D { let T { a, .. } = t; let o = Canonical::new("t"); o.digest() }',
        frozenset({"case::digest .."}),
    ),
    (
        'fn digest(T { a, b: _ }: T) -> D { let o = Canonical::new("t"); a.seal(&mut o); o.digest() }',
        frozenset({"case::digest b"}),
    ),
    (
        "fn make() -> impl Seal { struct L { a: u64, b: u64 } impl Seal for L { fn seal(&self, out: &mut Canonical) "
        "{ let Self { a, b: _ } = self; } } L { a: 1, b: 2 } }",
        frozenset({"case::L::seal b"}),
    ),
    (
        'fn read(t: &T) -> (u64, D) { fn tag() -> D { Canonical::new("t").digest() } '
        "let T { a, .. } = t; (*a, tag()) }",
        frozenset(),
    ),
    ('fn digest(&self) -> impl Sized { let Self { a: _ } = self; Canonical::new("t") }', frozenset({"case::digest a"})),
    ('fn tag(d: D) -> Digest { let D { key: _, v } = d; Digest::of_canonical(b"tag") }', frozenset()),
    ("impl R { fn seal(&mut self, d: D) { let D { a: _, b } = d; self.b = b; } }", frozenset()),
    (
        "fn outer() -> u8 { fn inner(d: D) -> Digest { let D { k: _, v } = d; Digest::of_canonical(&v) } 0 }",
        frozenset({"case::inner k"}),
    ),
)


def seal_cases() -> list[Violation]:
    out = []
    for text, want in SEAL_CASES:
        src = parse_source("crates/ng-case/src/case.rs", text)
        got = frozenset(f"{item} {hole}" for item, body in seal_scopes(src) for _, hole in seal_holes(body))
        if got != want:
            out.append(
                Violation(
                    "scan",
                    repr(text[:50]),
                    f"found {sorted(got) or 'nothing'}, the seal rule says {sorted(want) or 'nothing'}",
                )
            )
    return out


def _key(t: Token) -> tuple[str, str, bool]:
    return t.kind, t.written or t.value, t.joint


def _first_moved(was: list[Token] | tuple[Token, ...], now: list[Token] | tuple[Token, ...]) -> tuple[int, str] | None:
    for a, b in zip(was, now):
        if _key(a)[:2] != _key(b)[:2]:
            return b.line, f"`{_key(a)[1][:40]}` became `{_key(b)[1][:40]}`"
        if a.joint != b.joint:
            return b.line, f"the spacing after `{_key(b)[1][:40]}` changed"
    if len(now) > len(was):
        return now[len(was)].line, f"`{_key(now[len(was)])[1][:40]}` added"
    if len(was) > len(now):
        return (now[-1].line if now else 1), f"`{_key(was[len(now)])[1][:40]}` removed"
    return None


def _compiled(line: str) -> str:
    """rustdoc compiles a hidden `# ` line without its mark, a lone `#` as nothing, and `##` as `#`."""
    trimmed = line.strip()
    if trimmed.startswith("##"):
        return trimmed[1:]
    return trimmed[2:] if trimmed.startswith("# ") or trimmed == "#" else line


def _doctest_code(block: tuple[int, int, str, str]) -> list[Token] | str:
    code = "\n".join(_compiled(line) for line in block[2].split("\n"))
    try:
        return lex(code, block[1])[0]
    except LexError:
        return code


def comment_changes(was: Source, now: Source) -> list[tuple[str, int, str]]:
    out = []
    if moved := _first_moved(was.code, now.code):
        out.append(("code token", *moved))
    pairs = list(zip(was.docblocks, now.docblocks))
    if len(was.docblocks) != len(now.docblocks):
        line = next((b[0] for a, b in pairs if a[2:] != b[2:]), now.docblocks[-1][0] if now.docblocks else 1)
        return out + [("doctests", line, f"{len(was.docblocks)} doctests became {len(now.docblocks)}")]
    if fence := next(((b[0], f"{a[3]!r} became {b[3]!r}") for a, b in pairs if a[3] != b[3]), None):
        out.append(("doctest fence", *fence))
    for a, b in pairs:
        x, y = _doctest_code(a), _doctest_code(b)
        if isinstance(x, list) and isinstance(y, list):
            moved = _first_moved(x, y)
        else:
            moved = None if x == y else (b[0], "changed, and it does not lex, so its text was compared")
        if moved:
            out.append(("doctest code", *moved))
            break
    return out


MOVED_CODE, MOVED_DOCTEST, MOVED_FENCE, MOVED_COUNT = (
    frozenset({"code token"}),
    frozenset({"doctest code"}),
    frozenset({"doctest fence"}),
    frozenset({"doctests"}),
)
COMMENT_CASES: tuple[tuple[str, str, frozenset[str]], ...] = (
    ("// why\nfn f() {}", "// another why\nfn f() {}", frozenset()),
    ("/* a /* nested */ */ fn f() {}", "fn f() {}", frozenset()),
    ("/// Doc.\npub fn f() {}", "pub fn f() {}", frozenset()),
    ("fn f() { let x = 1; }", "fn f() {\n    let x = 1;\n}", frozenset()),
    (
        "/// ```\n/// // a\n/// let x = 1;\n/// ```\npub fn f() {}",
        "/// ```\n/// let x = 1;\n/// ```\npub fn f() {}",
        frozenset(),
    ),
    ("fn f() -> u8 { 1 }", "fn f() -> u8 { 2 }", MOVED_CODE),
    ('const S: &str = "// a";', 'const S: &str = "// b";', MOVED_CODE),
    ('fn f() -> usize { "a".len() }', 'fn f() -> usize { b"a".len() }', MOVED_CODE),
    ('fn f() -> usize { "\\n".len() }', 'fn f() -> usize { r"\\n".len() }', MOVED_CODE),
    ('#[doc = "a"]\npub fn f() {}', '#[doc = "b"]\npub fn f() {}', MOVED_CODE),
    (
        '/// ```compile_fail,E0308\n/// let _: u8 = "x";\n/// ```\npub fn f() {}',
        '/// ```compile_fail,E0277\n/// let _: u8 = "x";\n/// ```\npub fn f() {}',
        MOVED_FENCE,
    ),
    (
        "/// ```\n/// let x = 1;\n/// ```\npub fn f() {}",
        "/// ```\n/// let x = 2;\n/// ```\npub fn f() {}",
        MOVED_DOCTEST,
    ),
    (
        "/// Doc.\n///\n/// ```\n/// let x = 1;\n/// ```\npub fn f() {}",
        "/// Doc.\npub fn f() {}",
        MOVED_COUNT,
    ),
    (
        "/// ```\n/// let x = 1;\n/// ```\npub fn f() {}",
        "/// ```text\n/// let x = 1;\n/// ```\npub fn f() {}",
        MOVED_COUNT,
    ),
    ("fn f(a: bool, b: bool) -> bool { a && b }", "fn f(a: bool, b: bool) -> bool { a & &b }", MOVED_CODE),
    ("fn f() { x.iter().map(g); }", "fn f() {\n    x.iter()\n        .map(g);\n}", frozenset()),
    ('fn f() { m!("a"tag); }', 'fn f() { m!("a" tag); }', MOVED_CODE),
    ('fn f() { m!("a"℘); }', 'fn f() { m!("a" ℘); }', MOVED_CODE),
    ("fn f() { m!('r#foo); }", "fn f() { m!('r #foo); }", MOVED_CODE),
    ("/// ```\n/// #x\n/// ```\npub fn f() {}", "/// ```\n/// # x\n/// ```\npub fn f() {}", MOVED_DOCTEST),
    (
        "/// ```\n/// # // why\n/// let x = 1;\n/// ```\npub fn f() {}",
        "/// ```\n/// let x = 1;\n/// ```\npub fn f() {}",
        frozenset(),
    ),
    (
        "/**\n```\nlet b = &1;\nlet r =\n*b;\n```\n*/\npub fn f() {}",
        "/**\n```\nlet b = &1;\nlet r =\nb;\n```\n*/\npub fn f() {}",
        MOVED_DOCTEST,
    ),
    ("fn f() { m!(1.); }", "fn f() { m!(1 .); }", MOVED_CODE),
    ("fn f() -> u32 { 1.max(2) }", "fn f() -> u32 {\n    1\n        .max(2)\n}", frozenset()),
    (
        "/// ```\n/// let x = 1;\n/// ```\npub fn f() {}",
        "/// ```\n/// let x = 1;\n/// ````\npub fn f() {}",
        MOVED_FENCE,
    ),
    ("fn f(a: u8) -> u8 { a >/* c */>= 1 }", "fn f(a: u8) -> u8 { a >>= 1 }", MOVED_CODE),
    (
        "/// ```\n/// let x = 1;\n/// assert_eq!(x, 1);\n/// ```\n///\n/// ```\n/// let y = 2;\n/// ```\npub fn f() {}",
        "/// ```\n/// let x = 1;\n/// ```\n///\n/// ```\n/// assert_eq!(x, 1);\n/// let y = 2;\n/// ```\npub fn f() {}",
        MOVED_DOCTEST,
    ),
    (
        '/// ```compile_fail\n/// let x = 1; let s = "\n/// ```\npub fn f() {}',
        '/// ```compile_fail\n/// let x = 2; let s = "\n/// ```\npub fn f() {}',
        MOVED_DOCTEST,
    ),
)


def comment_cases() -> list[Violation]:
    out = []
    for old, new, want in COMMENT_CASES:
        path = "crates/ng-case/src/case.rs"
        got = {kind for kind, _, _ in comment_changes(parse_source(path, old), parse_source(path, new))}
        if got != want:
            out.append(
                Violation(
                    "comments",
                    f"{old[:30]!r} to {new[:30]!r}",
                    f"found {sorted(got) or 'nothing'}, the rules say {sorted(want) or 'nothing'}",
                )
            )
    return out


def _blob(rev: str, path: str) -> str:
    r = subprocess.run(["git", "-C", str(ROOT), "cat-file", "blob", f"{rev}:{path}"], capture_output=True)
    if r.returncode != 0:
        raise GateError(f"{rev}:{path}: {r.stderr.decode(errors='replace').strip()}")
    return r.stdout.decode("utf-8")


def comments_only(base_arg: str | None) -> int:
    base = resolve_base(ROOT, base_arg)[0] if base_arg else git(ROOT, "merge-base", "HEAD", "refs/remotes/origin/main")
    if base is None:
        raise GateError("no base: HEAD shares no history with origin/main; name one with --base")
    changes = changed_paths(ROOT, base)
    found = []
    if not changes:
        found.append(Violation("comments-only", base, "nothing changed; name the commit the change starts from"))
    rust = doctests = 0
    for path, (was, now) in sorted(changes.items()):
        if not path.endswith(".rs"):
            found.append(Violation("comments-only", path, "not a Rust file; the token proof covers .rs files only"))
        elif (was, now) != (REGULAR, REGULAR):
            found.append(
                Violation("comments-only", path, f"mode {was or 'absent'} became {now or 'absent'}; a comment edit "
                          "changes a file in place")
            )
        else:
            old, new = parse_source(path, _blob(base, path)), parse_source(path, _read(ROOT / path))
            found += [Violation("comments-only", f"{path}:{line}", f"{kind} {what}") for kind, line, what in
                      comment_changes(old, new)]
            rust, doctests = rust + 1, doctests + len(new.docblocks)
    for v in found:
        print(v)
    print(f"comments-only: {len(changes)} changed paths against {base}; {rust} Rust files compared, {doctests} "
          "doctests in them")
    return 1 if found else 0


PROBE_LOCK = (
    "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n"
)


def write_probe(name: str, source: str) -> pathlib.Path:
    """A throwaway crate under target/gate/, its own workspace, beneath the root so clippy finds clippy.toml.
    It carries the pinned toolchain itself, because `codes` copies it to CARGO_TARGET_DIR, outside the root."""
    d = ROOT / "target/gate" / name
    (d / "src").mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / "rust-toolchain.toml", d / "rust-toolchain.toml")
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


DOC_FENCE = re.compile(r"^(\s*//[/!] ?\s*)(```+|~~~+)(.*)$")
FAILURE = re.compile(r"^---- (\S+) - .*\(line (\d+)\) stdout ----$", re.M)
CODES_PROBE = """\
//! ```compile_fail,E0308
//! let _: u8 = "text";
//! ```
//! ```compile_fail,E0599
//! let _: u8 = "text";
//! ```
//! ```compile_fail,E0308
//! let _: u8 = 1;
//! ```
//! ```compile_fail
//! let _: u8 = "text";
//! ```
"""
CODES_PROBE_FLAGGED = {4, 7, 10}


def _fence_words(info: str) -> list[str]:
    return [w for w in re.split(r"[\s,]+", info) if w]


def compile_fail_codes(
    root: pathlib.Path, cwd: str, sources: list[str], packages: list[str]
) -> tuple[list[Violation], int, int]:
    """Stable rustdoc runs a compile_fail doctest without checking its code, so the doctests run again in a
    copy of the tree with the markers removed, and each must fail with the code its marker declares."""
    base = (ROOT / (os.environ.get("CARGO_TARGET_DIR") or "target")).resolve() / "gate/codes"
    tree = base / "tree"
    base.mkdir(parents=True, exist_ok=True)
    # An absolute CARGO_TARGET_DIR, as on the integrate gate, shares this tree between checkouts, so runs
    # take turns: otherwise one run's doctests could build from another checkout's copy.
    with open(base / "lock", "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        shutil.rmtree(tree, ignore_errors=True)
        for rel in list_files(root):
            if (root / rel).is_file() and not (root / rel).is_symlink():
                (tree / rel).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(root / rel, tree / rel)
        declared: dict[tuple[str, int], str | None] = {}
        for rel in sources:
            lines = _read(root / rel).split("\n")
            for n, line in enumerate(lines, 1):
                m = DOC_FENCE.match(line)
                if m and "compile_fail" in (words := _fence_words(m.group(3))):
                    codes = [w for w in words if re.fullmatch(r"E\d{4}", w)]
                    declared[(rel, n)] = codes[0] if len(codes) == 1 else None
                    kept = [w for w in words if w != "compile_fail" and not re.fullmatch(r"E\d{4}", w)]
                    lines[n - 1] = m.group(1) + m.group(2) + ",".join(kept)
            (tree / rel).write_text("\n".join(lines), encoding="utf-8")
        r = subprocess.run(
            ["cargo", "test", "--doc", "--locked", "--no-fail-fast", *(a for p in packages for a in ("-p", p)),
             "--target-dir", str(base / "target")],
            cwd=tree / cwd,
            env={**os.environ, "RUSTFLAGS": "", "RUSTDOCFLAGS": ""},
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
    if r.returncode != 0 and "Doc-tests" not in r.stdout:
        raise GateError(f"cargo test --doc in {cwd} built no doctests: {r.stdout.strip()[-2000:]}")
    sections = FAILURE.split(r.stdout)
    got = {
        (posixpath.normpath(posixpath.join(cwd, sections[i])), int(sections[i + 1])): set(
            re.findall(r"error\[(E\d{4})\]", sections[i + 2])
        )
        for i in range(1, len(sections), 3)
    }
    found, passed = [], 0
    for (rel, line), code in sorted(declared.items()):
        failed_with = got.pop((rel, line), set())
        if code is None:
            found.append(Violation("codes", f"{rel}:{line}", "declare the one error code the doctest fails with, "
                                   "as in compile_fail,E0308"))
        elif code not in failed_with:
            what = f"fails with {', '.join(sorted(failed_with))}" if failed_with else "compiles"
            found.append(Violation("codes", f"{rel}:{line}", f"declares {code} but {what} without the marker"))
        else:
            passed += 1
    found += [
        Violation("codes", f"{rel}:{line}", "a doctest fails here that the scan read no compile_fail marker for; "
                  "write it as a /// block")
        for rel, line in sorted(got)
    ]
    return found, len(declared), passed


def codes(sketch: str | None) -> int:
    if sketch is not None:
        sources = sorted(_rel(ROOT, p) for p in (ROOT / sketch / "src").rglob("*.rs"))
        found, count, passed = compile_fail_codes(ROOT, sketch, sources, [])
        if not count:
            found.append(Violation("codes", sketch, "no compile_fail doctests; the sketch's type rules are gone"))
        summary = f"codes: {sketch}: {count} compile_fail doctests, {passed} fail with their code"
    else:
        ws = load(ROOT)
        crates = os.environ["NG_GATE_CRATES"].split() if "NG_GATE_CRATES" in os.environ else sorted(ws.members)
        sources = sorted(
            s.path for c in crates for s in ws.members[c].sources if s.path.startswith(f"{ws.members[c].dir}/src/")
        )
        found, count, passed = compile_fail_codes(ROOT, ".", sources, crates) if crates else ([], 0, 0)
        summary = f"codes: {len(crates)} crates, {count} compile_fail doctests, {passed} fail with their code"
    if os.environ.get("NG_GATE_MODE") != "changed":
        flagged, _, _ = compile_fail_codes(write_probe("codes-probe", CODES_PROBE), ".", ["src/lib.rs"], [])
        lines = {int(v.subject.rsplit(":", 1)[1]) for v in flagged}
        if lines != CODES_PROBE_FLAGGED:
            found.append(
                Violation("codes", "the probe", f"flagged lines {sorted(lines)}; its bad codes are at "
                          f"{sorted(CODES_PROBE_FLAGGED)}")
            )
        summary += f"; probe: {len(lines & CODES_PROBE_FLAGGED)} of {len(CODES_PROBE_FLAGGED)} bad codes flagged"
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


@dataclasses.dataclass(frozen=True)
class SweepParts:
    first: tuple[str, ...]
    rest: tuple[str, ...]


@functools.cache
def mutate_py():
    spec = importlib.util.spec_from_file_location("mutate", ROOT / MUTATE)
    if spec is None or spec.loader is None:
        raise GateError(f"{MUTATE} does not load")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sweep_parts(
    mode: str, scope: str, base: str | None, rows: Sequence[dict], base_rows: Sequence[dict], changed: Set[str]
) -> SweepParts:
    if mode == "changed":
        return SweepParts((), ())
    whole = mode == "full" or scope == "everything" or base is None
    pool = [r for r in rows if whole or mutate_py().touched(r, changed)]
    was = {r["name"]: r for r in base_rows}
    first = {r["name"] for r in pool if base is not None and (r["path"] in changed or was.get(r["name"]) != r)}
    return SweepParts(
        tuple(r["name"] for r in pool if r["name"] in first), tuple(r["name"] for r in pool if r["name"] not in first)
    )


def rows_at(rev: str) -> list[dict]:
    if not _git_z(ROOT, "ls-tree", "-z", "--name-only", rev, "--", MUTANTS):
        return []
    try:
        table = tomllib.loads(_blob(rev, MUTANTS))
    except tomllib.TOMLDecodeError as e:
        raise GateError(f"{rev}:{MUTANTS}: {e}") from None
    return [{**row, "test": row.get("test", table.get("test"))} for row in table.get("mutant", [])]


VERDICTS = frozenset({"CAUGHT", "CAUGHT at build", "SURVIVED", "NOT APPLIED", "UNCLEAR"})
VERDICT_LINE = re.compile(r"(.{15})  (\S+)  \d+ s(?:  |$)")


def first_stop(lines: Iterable[str], names: Sequence[str], later: int) -> tuple[tuple[str, str] | None, int]:
    judged = set()
    for line in lines:
        if (m := VERDICT_LINE.match(line)) and (verdict := m[1].rstrip()) in VERDICTS:
            judged.add(m[2])
            if verdict not in ("CAUGHT", "CAUGHT at build"):
                return (m[2], verdict), len(set(names) - judged) + later
    return None, len(set(names) - judged) + later


def sweep(argv: list[str], names: Sequence[str], later: int) -> int:
    def echoed(stream: Iterable[bytes]) -> Iterable[str]:
        for raw in stream:
            line = raw.decode(errors="replace")
            sys.stdout.write(line)
            sys.stdout.flush()
            yield line.rstrip("\n")

    with subprocess.Popen(argv, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT) as child:
        stop, left = first_stop(echoed(child.stdout), names, later)
        if stop is not None:
            child.kill()
    if stop is not None:
        print(f"40-mutants: the sweep stops at {stop[0]}, {stop[1]}. Rows not run: {left}", flush=True)
        return 1
    if child.returncode != 0:
        print(f"40-mutants: the sweep failed. Rows not run: {left}", flush=True)
        return 1
    return 0


def mutants(part: str | None) -> int:
    mode = os.environ.get("NG_GATE_MODE") or "full"
    scope = os.environ.get("NG_GATE_SCOPE") or "everything"
    base = os.environ.get("NG_GATE_BASE") or "-"
    base = None if base == "-" else base
    command = [sys.executable, "-I", MUTATE, MUTANTS]
    rows = mutate_py().load_rows(ROOT / MUTANTS, [])
    changed: set[str] = set()
    base_rows: list[dict] = []
    if mode != "changed" and base is not None:
        changed, base_rows = set(_git_z(ROOT, "diff", "--name-only", "-z", base, "HEAD")), rows_at(base)
    parts = sweep_parts(mode, scope, base, rows, base_rows, changed)
    if part != "rest" and sweep([*command, "--check"], (*parts.first, *parts.rest), 0):
        return 1
    if part != "rest":
        named = f"{len(parts.first)} rows: {' '.join(parts.first)}" if parts.first else "no row"
        print(f"40-mutants: first part, {named}", flush=True)
        if parts.first and sweep([*command, "--only", *parts.first], parts.first, len(parts.rest)):
            return 1
    if part != "first" and mode != "changed":
        print(f"40-mutants: rest part, {len(parts.rest)} rows", flush=True)
        only = [] if len(parts.rest) == len(rows) else ["--only", *parts.rest]
        tested = ["--tested", str(ROOT / TESTED)] if part == "rest" else []
        if parts.rest and sweep([*command, *tested, *only], parts.rest, 0):
            return 1
    return 0


def tested(args: list[str]) -> int:
    record = ROOT / TESTED
    if args == ["--reset"]:
        record.unlink(missing_ok=True)
        return 0
    if git(ROOT, "status", "--porcelain") != "":
        print("10-rust: the checkout differs from HEAD, so the sweep runs every baseline")
        return 0
    entry = {"commit": git(ROOT, "rev-parse", "HEAD"), "settings": mutate_py().build_settings(os.environ), "args": args}
    record.parent.mkdir(parents=True, exist_ok=True)
    with record.open("a", encoding="utf-8") as file:
        file.write(json.dumps(entry) + "\n")
    return 0


PART_ROWS = (
    {"name": "added", "path": "crates/ng-a/src/one.rs", "old": "a", "new": "b", "test": "-p ng-a --test alpha"},
    {"name": "pathed", "path": "crates/ng-b/src/two.rs", "old": "a", "new": "b", "test": "-p ng-b --test beta"},
    {"name": "edited", "path": "crates/ng-a/src/three.rs", "old": "a", "new": "c", "test": "-p ng-a --test alpha"},
    {"name": "idle", "path": "crates/ng-b/src/four.rs", "old": "a", "new": "b", "test": "-p ng-b --test beta"},
    {"name": "tested", "path": "crates/ng-a/src/five.rs", "old": "a", "new": "b", "test": "-p ng-a --test alpha"},
)
PART_BASE = (
    PART_ROWS[1],
    {**PART_ROWS[2], "new": "b"},
    *PART_ROWS[3:],
    {"name": "gone", "path": "crates/ng-a/src/six.rs", "old": "a", "new": "b", "test": "-p ng-a --test alpha"},
)
PART_CHANGED = frozenset({"crates/ng-b/src/two.rs", "crates/ng-a/tests/alpha.rs"})
EVERY_ROW = frozenset(r["name"] for r in PART_ROWS)
PART_CASES: tuple[tuple[str, str, str | None, frozenset[str], tuple[str, ...]], ...] = (
    ("changed", "affected", "base", frozenset(), ()),
    ("full", "everything", None, EVERY_ROW, ()),
    ("merge", "everything", None, EVERY_ROW, ()),
    ("merge", "everything", "base", EVERY_ROW, ("added", "pathed", "edited")),
    ("merge", "affected", "base", EVERY_ROW - {"idle"}, ("added", "pathed", "edited")),
)


def part_cases() -> list[Violation]:
    out = []
    for mode, scope, base, pool, first in PART_CASES:
        rest = tuple(r["name"] for r in PART_ROWS if r["name"] in pool - set(first))
        got = sweep_parts(mode, scope, base, PART_ROWS, PART_BASE, PART_CHANGED)
        if got != SweepParts(first, rest):
            out.append(
                Violation(
                    "sweep",
                    f"{mode} {scope} base {base or '-'}",
                    f"first {' '.join(got.first) or 'none'} and rest {' '.join(got.rest) or 'none'}; the pool splits "
                    f"into first {' '.join(first) or 'none'} and rest {' '.join(rest) or 'none'}",
                )
            )
    return out


SWEEP_OF = "sweep of 0123abc, 3 rows, budget 420 s"
BASELINE = "baseline  12 s  cargo test -p ng-a --test alpha"
LINE_CASES: tuple[tuple[tuple[str, ...], tuple[str, str] | None, int], ...] = (
    (
        (
            SWEEP_OF,
            BASELINE,
            f"{'CAUGHT':<15}  a  3 s  alpha::x",
            f"{'CAUGHT at build':<15}  b  1 s  error[E0308]: mismatched types",
            f"{'CAUGHT':<15}  c  4 s  alpha::y",
            "summary: 2 CAUGHT, 1 CAUGHT at build, of 3 rows at 0123abc",
        ),
        None,
        2,
    ),
    (
        (
            SWEEP_OF,
            BASELINE,
            f"{'CAUGHT':<15}  a  3 s  alpha::x",
            f"{'SURVIVED':<15}  b  3 s",
            f"{'CAUGHT':<15}  c  4 s  alpha::y",
        ),
        ("b", "SURVIVED"),
        3,
    ),
    ((SWEEP_OF, BASELINE, f"{'UNCLEAR':<15}  a  420 s  over 420 s"), ("a", "UNCLEAR"), 4),
    ((SWEEP_OF, f"{'NOT APPLIED':<15}  c  0 s  0 matches in crates/ng-a/src/one.rs"), ("c", "NOT APPLIED"), 4),
    (("no row is touched after 0123abc",), None, 5),
    ((SWEEP_OF, "mutate: `cargo test -p ng-a --test alpha` must pass some test on the unmutated tree"), None, 5),
)


def line_cases() -> list[Violation]:
    out = []
    for lines, stop, left in LINE_CASES:
        got = first_stop(lines, ("a", "b", "c"), 2)
        if got != (stop, left):
            out.append(
                Violation(
                    "sweep",
                    repr(lines[-1][:40]),
                    f"stops at {got[0]} with {got[1]} rows not run; the driver's rule says {stop} with {left}",
                )
            )
    return out


FAKE_SWEEP = """\
import os, time
print(f"sweep of {os.getpid()}, 3 rows, budget 1 s", flush=True)
print(f"{'CAUGHT':<15}  a  1 s  alpha::x", flush=True)
print(f"{'SURVIVED':<15}  b  1 s", flush=True)
time.sleep(60)
"""
FAILED_SWEEP = """\
print("sweep of 0123abc, 3 rows, budget 1 s", flush=True)
print("baseline  1 s  cargo test -p ng-a --test alpha", flush=True)
raise SystemExit("mutate: another sweep is running; no sweep")
"""


def survivor_case() -> list[Violation]:
    log = io.StringIO()
    began = time.monotonic()
    with contextlib.redirect_stdout(log):
        code = sweep([sys.executable, "-I", "-c", FAKE_SWEEP], ("a", "b", "c"), 0)
    took = time.monotonic() - began
    out = []
    if code != 1 or took >= 10:
        out.append(Violation("sweep", "a fake sweep", f"returned {code} after {took:.0f} s; a SURVIVED row stops it"))
    if "40-mutants: the sweep stops at b, SURVIVED. Rows not run: 1" not in log.getvalue().splitlines():
        out.append(Violation("sweep", "a fake sweep", "no line reports b SURVIVED with 1 row not run"))
    if found := re.search(r"^sweep of (\d+),", log.getvalue(), re.M):
        with contextlib.suppress(ProcessLookupError):
            os.kill(int(found[1]), 0)
            out.append(Violation("sweep", "a fake sweep", "the fake still ran after the driver returned"))
    else:
        out.append(Violation("sweep", "a fake sweep", "the driver did not echo the fake's lines"))
    return out


def failed_sweep_case() -> list[Violation]:
    log = io.StringIO()
    with contextlib.redirect_stdout(log):
        code = sweep([sys.executable, "-I", "-c", FAILED_SWEEP], ("a", "b", "c"), 2)
    if code != 1 or "40-mutants: the sweep failed. Rows not run: 5" not in log.getvalue().splitlines():
        return [Violation("sweep", "a sweep that dies", f"returned {code}; it fails with 5 rows not run")]
    return []


def check_case() -> list[Violation]:
    out = []
    with tempfile.TemporaryDirectory(prefix="gate-sweep-") as tmp:
        table = pathlib.Path(tmp) / "mutants.toml"
        rows = [f'[[mutant]]\nname = "{n}"\npath = "crates/ng-none/src/{n}.rs"\nold = "x"\nnew = "y"\n' for n in "ab"]
        table.write_text("\n".join(['test = "-p ng-none --test none"\n', *rows]), encoding="utf-8")
        log = io.StringIO()
        with contextlib.redirect_stdout(log):
            code = sweep([sys.executable, "-I", MUTATE, str(table), "--check"], ("a", "b"), 0)
    stopped = "40-mutants: the sweep stops at a, NOT APPLIED. Rows not run: 1" in log.getvalue().splitlines()
    if code != 1 or not stopped:
        out.append(Violation("sweep", f"{MUTATE} --check", "the driver misses mutate.py's own NOT APPLIED line"))
    return out


SHA, OTHER_SHA = "1" * 40, "2" * 40
SETTINGS = {"RUSTFLAGS": ""}
TESTED_RECORD = (
    (SHA, SETTINGS, ["-p", "ng-a", "-p", "ng-b", "--", "--include-ignored"]),
    (SHA, SETTINGS, ["-p", "ng-c", "--lib", "--test", "beta"]),
    (SHA, SETTINGS, ["-p", "ng-f", "--test", "alpha", "only_this_one"]),
    (OTHER_SHA, SETTINGS, ["-p", "ng-d"]),
    (SHA, {"RUSTFLAGS": "-Cdebug-assertions=off"}, ["-p", "ng-e"]),
)
TESTED_CASES: tuple[tuple[str, bool, str], ...] = (
    ("-p ng-a --test alpha a_filter", True, "a crate 10-rust tested whole"),
    ("-p ng-b --lib a_filter", True, "the library of a crate 10-rust tested whole"),
    ("--package=ng-a -png-b --test=alpha", True, "two crates spelled the other ways"),
    ("-p ng-c --test beta a_filter", True, "a test target 10-rust named"),
    ("-p ng-c --lib", True, "the library 10-rust named"),
    ("--release -p ng-a --test alpha", False, "a --release baseline"),
    ("--doc -p ng-a Thing", False, "a doctest, which only a run builds"),
    ("-p ng-a --test alpha -- --ignored", False, "a test's own runner flags"),
    ("-p ng-c --test gamma", False, "a test target 10-rust did not name"),
    ("-p ng-a -p ng-c --test gamma", False, "two crates, one of them short of the target"),
    ("-p ng-z --test alpha", False, "a crate the plan did not test"),
    ("-p ng-d --test alpha", False, "a record from another commit"),
    ("-p ng-e --test alpha", False, "a record from other build settings"),
    ("-p ng-f --test alpha", False, "a call that ran only some tests"),
)


def tested_cases() -> list[Violation]:
    out = []
    mutate = mutate_py()
    with tempfile.TemporaryDirectory(prefix="gate-tested-") as tmp:
        record = pathlib.Path(tmp) / "tested.jsonl"
        missing = mutate.passed_calls(str(record), SHA, SETTINGS)
        lines = [json.dumps({"commit": c, "settings": s, "args": a}) for c, s, a in TESTED_RECORD]
        record.write_text("\n".join(lines) + "\n", encoding="utf-8")
        calls = mutate.passed_calls(str(record), SHA, SETTINGS)
    for test, skips, what in TESTED_CASES:
        if mutate.covered(test, calls) != skips:
            out.append(Violation("baseline", what, f"`{test}` {'runs' if skips else 'skips'} its baseline"))
    if missing or mutate.covered(TESTED_CASES[0][0], missing):
        out.append(Violation("baseline", "a missing record", "it skips a baseline"))
    scrubs = []
    for step in ("10-rust", "40-mutants"):
        text = _read(ROOT / STEPS / f"{step}.sh")
        scrubs.append(text[text.find('export RUSTFLAGS=""') : text.find("UPDATE_EXPECT\n") + len("UPDATE_EXPECT")])
    if not scrubs[0].startswith("export") or scrubs[0] != scrubs[1]:
        out.append(Violation("baseline", "40-mutants.sh", "it clears the build settings 10-rust.sh clears, line for line"))
    return out


DRIVER_CASES: tuple[Callable[[], list[Violation]], ...] = (survivor_case, failed_sweep_case, check_case, tested_cases)
ORDER_CASES: tuple[tuple[tuple[str, ...], str], ...] = (
    (("01-toolchain.sh", "99-unplaced.sh"), "verify: tools/verify.d/99-unplaced.sh has no place in the run order"),
    (("01-toolchain.sh", "40-mutants.sh"), "verify: the plan failed"),
)


def order_cases() -> list[Violation]:
    out = []
    for files, last in ORDER_CASES:
        with tempfile.TemporaryDirectory(prefix="gate-order-") as tmp:
            steps = pathlib.Path(tmp) / "tools" / "verify.d"
            steps.mkdir(parents=True)
            shutil.copyfile(ROOT / "tools" / "verify", steps.parent / "verify")
            for name in files:
                (steps / name).write_text("exit 0\n", encoding="utf-8")
            run = subprocess.run(["bash", str(steps.parent / "verify")], capture_output=True, text=True)
        lines = run.stdout.splitlines()
        if run.returncode != 1 or lines[-1:] != [last]:
            out.append(
                Violation(
                    "order",
                    " ".join(files),
                    f"exit {run.returncode} with last line {lines[-1:]}; tools/verify exits 1 with {last!r}",
                )
            )
    return out


def seam() -> int:
    ws = load(ROOT)
    found = check(ws, "registration") + check(ws, "seam")
    for v in found:
        print(v)
    probes, bites = run_probes(ws, tree_passes=not found)
    cases = plan_cases(ws) + scan_cases() + comment_cases() + seal_cases()
    cases += part_cases() + line_cases() + [v for case in DRIVER_CASES for v in case()] + order_cases()
    for v in probes + cases:
        print(v)
    print(
        f"05-seam: {len(ws.members)} members against the crate table; {len(RULES)} rules, {bites} probes bite; "
        f"{len(PLAN_CASES)} plan cases; {len(SCAN_CASES)} scan cases; {len(COMMENT_CASES)} comment cases; "
        f"{len(SEAL_CASES)} seal cases; {len(PART_CASES)} part cases; {len(LINE_CASES)} line cases; "
        f"{len(DRIVER_CASES)} driver cases; {len(ORDER_CASES)} order cases; "
        f"{len(must_fail_rows(_read(ROOT / CONTRACT)))} must-fail rows"
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
            case ["mutants"]:
                return mutants(None)
            case ["mutants", "first" | "rest" as part]:
                return mutants(part)
            case ["tested", "--reset"]:
                return tested(["--reset"])
            case ["tested", "--", *args]:
                return tested(args)
            case ["bans"]:
                return bans()
            case ["f64"]:
                return f64()
            case ["doctests"]:
                return doctests()
            case ["codes"]:
                return codes(None)
            case ["codes", sketch]:
                return codes(sketch)
            case ["comments-only"]:
                return comments_only(None)
            case ["comments-only", "--base", base]:
                return comments_only(base)
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
