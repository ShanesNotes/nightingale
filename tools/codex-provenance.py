"""Prints the model and reasoning effort each Codex pass ran with, read from its rollout.

    python3 -I tools/codex-provenance.py PASS [PASS ...]

Python 3.11 stdlib only. A PASS is one Codex run's saved output, whose header at the top names its `session id:`,
or that id; an id quoted anywhere else in the log is not read. Its rollout is the file under $CODEX_HOME/sessions,
default ~/.codex/sessions, whose name ends in the id. Each pass prints every model and effort its turns ran with, its
session, start, Codex version and the commit Codex saw at the start, then the models and efforts of its helper
sessions: the rollouts that name it, or one of its helpers, as parent. `codex review` runs its turns in a helper, so
its own session shows no turn. A forked session's rollout repeats the turns of the session it forked from, and only
its own count. Nothing else of a rollout prints: no prompt, path or account.

Exit 0 when every pass's session or helpers record a turn, every turn records an id, a model and an effort, every
rollout reads whole, every forked session's source rollout is there and whole, and every helper the pass spawned has a
rollout; 1 otherwise; 2 usage.
"""
import json
import os
import pathlib
import re
import sys

SESSION_ID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
SESSIONS = pathlib.Path(os.environ.get("CODEX_HOME") or pathlib.Path.home() / ".codex") / "sessions"


def first_meta(path):
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            entry = json.loads(handle.readline())
    except (OSError, ValueError):
        return {}
    payload = entry.get("payload") if isinstance(entry, dict) and entry.get("type") == "session_meta" else None
    return payload if isinstance(payload, dict) else {}


def text_of(value):
    return value if isinstance(value, str) and value else None


def dict_of(value):
    return value if isinstance(value, dict) else {}


def header_session(text):
    lines = text.splitlines()
    k = 0
    while k < len(lines) and lines[k] in ("", "Reading additional input from stdin..."):
        k += 1
    if not lines[k : k + 1] or not lines[k].startswith("OpenAI Codex v") or lines[k + 1 : k + 2] != ["--------"]:
        return None
    for line in lines[k + 2 :]:
        if line == "--------":
            return None
        if found := re.fullmatch(r"session id: (" + SESSION_ID.pattern + ")", line):
            return found.group(1)
    return None


def read(path):
    meta, turns, calls, spawned, whole = None, [], set(), set(), True
    try:
        handle = open(path, encoding="utf-8", errors="replace")
    except OSError:
        return {}, [], set(), False
    with handle:
        for line in handle:
            if not line.strip():
                continue
            try:
                entry = json.loads(line)
            except ValueError:
                whole = False
                continue
            kind = entry.get("type") if isinstance(entry, dict) else None
            payload = entry.get("payload") if isinstance(entry, dict) else None
            if not isinstance(payload, dict):
                whole = whole and kind not in ("session_meta", "turn_context")
                continue
            if kind == "session_meta" and meta is None:
                meta = payload
            elif kind == "turn_context":
                settings = dict_of(dict_of(payload.get("collaboration_mode")).get("settings"))
                turn_id = text_of(payload.get("turn_id"))
                model = text_of(payload.get("model")) or text_of(settings.get("model"))
                effort = text_of(payload.get("effort")) or text_of(settings.get("reasoning_effort"))
                whole = whole and turn_id is not None
                turns.append((turn_id, model, effort))
            elif payload.get("type") == "function_call" and payload.get("name") == "spawn_agent":
                calls.add(text_of(payload.get("call_id")))
            elif payload.get("type") == "function_call_output" and text_of(payload.get("call_id")) in calls:
                output = payload.get("output")
                try:
                    task = json.loads(output) if isinstance(output, str) else output
                except ValueError:
                    task = None
                if name := text_of(dict_of(task).get("task_name")):
                    spawned.add(name)
    return meta or {}, turns, spawned, whole


def agent_path(meta):
    spawn = dict_of(dict_of(meta.get("source")).get("subagent"))
    return text_of(dict_of(spawn.get("thread_spawn")).get("agent_path"))


def shown(pairs):
    return " and ".join(f"{model or '?'} {effort or '?'}" for model, effort in pairs) or "no turn"


def main(args):
    if not args:
        print(__doc__, file=sys.stderr)
        return 2
    rollouts = {p.name[-42:-6]: p for p in sorted(SESSIONS.rglob("rollout-*.jsonl"))}
    metas = {session: first_meta(path) for session, path in rollouts.items()}
    children = {}
    for session, meta in metas.items():
        children.setdefault(text_of(meta.get("parent_thread_id")), []).append(session)
    cache = {}

    def load(session):
        if session not in cache:
            cache[session] = read(rollouts[session])
        return cache[session]

    def own_pairs(session):
        source = text_of(metas[session].get("forked_from_id"))
        if source and (source not in rollouts or not load(source)[3]):
            return None
        inherited = {turn[0] for turn in load(source)[1]} if source else set()
        return [(model, effort) for turn_id, model, effort in load(session)[1] if turn_id not in inherited]

    failed = False
    for arg in args:
        if os.path.isfile(arg):
            label = os.path.basename(arg)
            try:
                session = header_session(pathlib.Path(arg).read_text(encoding="utf-8", errors="replace"))
            except OSError as error:
                print(f"{label}: cannot be read ({error.strerror})")
                failed = True
                continue
        elif SESSION_ID.fullmatch(arg):
            label, session = arg, arg
        else:
            print(f"{os.path.basename(arg)}: neither a file nor a session id", file=sys.stderr)
            return 2
        if session is None or session not in rollouts:
            why = "no `session id:` in its header" if session is None else f"session {session} has no rollout"
            print(f"{label}: {why}")
            failed = True
            continue
        helpers, todo = [], list(children.get(session, []))
        while todo:
            helper = todo.pop()
            if helper != session and helper not in helpers:
                helpers.append(helper)
                todo += children.get(helper, [])
        meta, _, spawned, whole = load(session)
        own = {s: own_pairs(s) for s in (session, *helpers)}
        orphaned = [s for s, p in own.items() if p is None]
        pairs = list(dict.fromkeys(own[session] or []))
        helper_pairs = list(dict.fromkeys(pair for helper in helpers for pair in own[helper] or []))
        spawned = spawned.union(*(load(helper)[2] for helper in helpers))
        whole = whole and all(load(helper)[3] for helper in helpers)
        family = {session, text_of(metas[session].get("parent_thread_id")), *helpers}
        missing = spawned - {agent_path(metas[child]) for parent in family for child in children.get(parent, [])}
        started = re.sub(r"\.\d+Z$", "Z", text_of(meta.get("timestamp")) or "unknown")
        commit = text_of(dict_of(meta.get("git")).get("commit_hash")) or "unknown"
        line = f"{label}: {shown(pairs)}; session {session}, started {started}, "
        line += f"codex {text_of(meta.get('cli_version')) or 'unknown'}, commit {commit}"
        line += f"; {len(helpers)} helper sessions: {shown(helper_pairs)}" if helpers else "; no helper rollouts"
        line += f"; {len(missing)} spawned helpers have no rollout" if missing else ""
        line += f"; {len(orphaned)} forked sessions lack a whole rollout to fork from" if orphaned else ""
        line += "" if whole else "; a rollout does not read whole"
        print(line)
        turns = pairs + helper_pairs
        failed |= bool(missing or orphaned) or not whole or not turns or any(None in pair for pair in turns)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
