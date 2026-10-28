#!/usr/bin/env bash
# Docs hygiene over every Markdown file outside .scratch/. docs/governance.md states each rule this
# step holds; a finding prints as <file>:<line>: <rule>: <text>. Permalink URLs are exempt.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 -I - <<'PY'
import pathlib
import posixpath
import re
import subprocess
import sys

listed = subprocess.run(['git', 'ls-files', '-co', '--exclude-standard'], capture_output=True, text=True,
                        check=True).stdout.splitlines()
tree = {f for f in listed if pathlib.Path(f).exists()}
dirs = set()
for f in tree:
    p = pathlib.PurePosixPath(f).parent
    while str(p) != '.':
        dirs.add(str(p))
        p = p.parent
docs = sorted(f for f in tree if f.endswith('.md') and not f.startswith('.scratch/'))

# Prose files must sit where docs/governance.md's allowlist puts them.
ALLOWED = ('README.md', 'AGENTS.md', 'CLAUDE.md', 'GLOSSARY.md', 'docs/srs.md', 'docs/governance.md', 'docs/adr/',
           'docs/agents/', 'docs/atlas/', 'docs/architecture/solver-contract.md',
           'docs/architecture/solver-contract-sketch/', '.agents/skills/')
CORE = ('docs/srs.md', 'docs/architecture/', 'docs/atlas/', 'docs/adr/')
# First segments that name a repo-root path, here or in pulse. A code span starting with one must exist.
ROOTS = {'docs', 'tools', 'crates', 'workers', 'apps', 'packages', 'seed', 'evals', 'infra', 'reference',
         '.agents', '.claude', '.github', '.scratch'}
ROOT_FILES = {'AGENTS.md', 'README.md', 'CLAUDE.md', 'GLOSSARY.md', 'CONTEXT.md', 'rust-toolchain.toml',
              'clippy.toml', 'rustfmt.toml', 'Cargo.toml', 'Cargo.lock'}
LINK = re.compile(r'\]\(([^)\s]+)(?:\s+"[^"]*")?\)')
URL = re.compile(r'https?://[^\s)>\]`]+')
FENCE = re.compile(r'^(```|~~~)[^\n]*\n.*?^\1[^\n]*$', re.S | re.M)
SPAN = re.compile(r'`([^`\n]+)`')
HEADING = re.compile(r'^#{1,6}\s+(?:[\d.]+\s+)?(status|current state|progress|next|todo|changelog|session|open)\b',
                     re.I | re.M)
PULSE_RECORD = re.compile(r'\[([^\]]*)\]\(https://github\.com/ShanesNotes/nightingale-pulse/blob/[^)]*'
                          r'/docs/decisions/\d{4}-[^)]*\)')

findings = []


def blank(m):
    return re.sub(r'[^\n]', ' ', m.group(0))


def report(path, text, pos, rule, excerpt):
    findings.append(f'{path}:{text.count(chr(10), 0, pos) + 1}: {rule}: {excerpt.strip().splitlines()[0][:100]}')


def exists_from(doc_dir, path):
    """A code-formatted path counts when its first segment names a repo root, or a directory beside
    the doc or above it; it must then exist from there. Other spans are not paths this check can place."""
    first = path.split('/', 1)[0]
    if first in ROOTS:
        bases = ['']
    else:
        bases, d = [], pathlib.PurePosixPath(doc_dir)
        while True:
            base = '' if str(d) == '.' else str(d) + '/'
            if (base + first) in dirs:
                bases.append(base)
                break
            if str(d) == '.':
                break
            d = d.parent
        if not bases:
            return True
    # A glob or brace set is checked up to the last directory before it.
    stem = re.split(r'[*{]', path, maxsplit=1)[0]
    if stem != path:
        stem = stem.rsplit('/', 1)[0]
    target = (bases[0] + stem).rstrip('/')
    target = str(pathlib.PurePosixPath(target)) if target else '.'
    return target in tree or target in dirs


for doc in docs:
    raw = open(doc, encoding='utf-8').read()
    no_urls = URL.sub(blank, raw)
    no_fence = FENCE.sub(blank, no_urls)
    prose = SPAN.sub(blank, no_fence)
    if not doc.startswith(ALLOWED):
        report(doc, raw, 0, 'outside the allowlist', doc)
    for m in LINK.finditer(FENCE.sub(blank, raw)):
        target = m.group(1)
        if re.match(r'^[a-z]+:|^#', target):
            continue
        resolved = posixpath.normpath(posixpath.join(posixpath.dirname(doc), target.split('#', 1)[0]))
        if resolved not in tree and resolved not in dirs:
            report(doc, raw, m.start(), 'dead link', target)
    for m in SPAN.finditer(no_fence):
        span = re.sub(r':[\d,–\-]+$', '', m.group(1).strip())
        if ' ' in span or '<' in span or span.startswith(('~', '/', '-', '$')):
            continue
        if '/' in span:
            if not exists_from(str(pathlib.PurePosixPath(doc).parent), span):
                report(doc, raw, m.start(), 'dead path', span)
        elif span in ROOT_FILES and span not in tree:
            report(doc, raw, m.start(), 'dead path', span)
    for m in re.finditer(r'\bvitals-', no_urls):
        report(doc, raw, m.start(), 'pulse crate name', no_urls[m.start():m.start() + 30])
    for m in HEADING.finditer(no_fence):
        report(doc, raw, m.start(), 'status heading', m.group(0))
    for m in re.finditer(r'\bStatus\W{0,2}:', prose):
        report(doc, raw, m.start(), 'status text', prose[m.start():m.start() + 40])
    for m in re.finditer(r'^\|[^|\n]*\bstatus\s*\|', prose, re.I | re.M):
        report(doc, raw, m.start(), 'status row', m.group(0))
    for m in re.finditer(r'\b\d{4}-\d{2}-\d{2}\b', prose):
        report(doc, raw, m.start(), 'date', m.group(0))
    for m in re.finditer(r'\b(TODO|FIXME|TBD)\b', prose):
        report(doc, raw, m.start(), 'todo', m.group(0))
    for m in re.finditer('\u2014', no_urls):
        report(doc, raw, m.start(), 'em dash', no_urls[max(0, m.start() - 20):m.start() + 20])
    if doc.startswith(CORE) or '/' not in doc:
        for m in re.finditer(r'wasm', no_urls, re.I):
            report(doc, raw, m.start(), 'other build target', no_urls[max(0, m.start() - 20):m.start() + 20])
    for m in PULSE_RECORD.finditer(raw):
        if not m.group(1).startswith('pulse '):
            report(doc, raw, m.start(), 'pulse record cited without "pulse"', m.group(1))

caps = {'AGENTS.md': 60, 'README.md': 40}
for doc, cap in caps.items():
    if doc in tree and (n := open(doc, encoding='utf-8').read().count('\n')) > cap:
        findings.append(f'{doc}:{n}: over its {cap}-line cap')


# GLOSSARY.md carries every SRS section 2 definition word for word, plus its Avoid words.
def srs_terms(text):
    body = text.split('\n## 2. Definitions\n', 1)[1].split('\n## 3.', 1)[0]
    terms = {}
    for m in re.finditer(r'^- ([^:\n]+): (.+)$', body, re.M):
        definition, _, avoid = m.group(2).partition(' Avoid: ')
        terms[m.group(1)] = (definition, {w.strip(' .') for w in avoid.split(',') if w.strip(' .')})
    return terms


def glossary_terms(text):
    terms = {}
    for m in re.finditer(r'^\*\*([^*\n]+)\*\*:\n((?:[^\n]+\n?)+)', text, re.M):
        lines = m.group(2).strip().split('\n')
        avoid = {w.strip(' .') for line in lines if line.startswith('_Avoid_:')
                 for w in line[len('_Avoid_:'):].split(',') if w.strip(' .')}
        definition = ' '.join(line for line in lines if not line.startswith('_Avoid_:'))
        terms[m.group(1)] = (definition, avoid)
    return terms


def norm(s):
    s = ' '.join(s.split())
    return s[:1].upper() + s[1:]


if 'docs/srs.md' in tree and 'GLOSSARY.md' in tree:
    glossary = glossary_terms(open('GLOSSARY.md', encoding='utf-8').read())
    for term, (definition, avoid) in srs_terms(open('docs/srs.md', encoding='utf-8').read()).items():
        entry = glossary.get(term)
        if entry is None:
            findings.append(f'GLOSSARY.md:1: SRS term missing: {term}')
        elif norm(entry[0]) != norm(definition) or not avoid <= entry[1]:
            findings.append(f'GLOSSARY.md:1: differs from SRS section 2: {term}')

for line in findings:
    print(line)
print(f'10-docs: {len(docs)} files, {len(findings)} findings')
sys.exit(1 if findings else 0)
PY
