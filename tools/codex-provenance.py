"""Prints the model and reasoning effort each Codex pass ran with, read from its rollout.

    python3 -I tools/codex-provenance.py PASS [PASS ...]

Python 3.11 stdlib only. A PASS is one Codex run's saved output, whose header names its `session id:`, or that id.
Its rollout is the file under $CODEX_HOME/sessions, default ~/.codex/sessions, whose name ends in the id. Each pass
prints every model and effort its turns ran with, its session, start, Codex version and the commit Codex saw at the
start, then the models and efforts of its helper sessions: the rollouts that name it, or one of its helpers, as
parent. `codex review` runs its turns in a helper, so its own session shows no turn. A forked helper's rollout
repeats its parent's turns, and only its own count. Nothing else of a rollout prints: no prompt, path or account.

Exit 0 when every pass's session or helpers record a turn, every turn records a model and an effort, every record
parses and every helper the pass spawned has a rollout; 1 otherwise; 2 usage.
"""
import json
import os
import pathlib
import re
import sys

SESSION_ID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
SESSIONS = pathlib.Path(os.environ.get("CODEX_HOME") or pathlib.Path.home() / ".codex") / "sessions"


def first_meta(path):
    with open(path, encoding="utf-8", errors="replace") as handle:
        try:
            entry = json.loads(handle.readline())
        except ValueError:
            return {}
    payload = entry.get("payload") if isinstance(entry, dict) and entry.get("type") == "session_meta" else None
    return payload if isinstance(payload, dict) else {}


def read(path):
    meta, turns, calls, spawned, whole = None, [], set(), set(), True
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if not line.strip():
                continue
            try:
                entry = json.loads(line)
            except ValueError:
                whole = False
                continue
            payload = entry.get("payload") if isinstance(entry, dict) else None
            if not isinstance(payload, dict):
                continue
            if entry.get("type") == "session_meta" and meta is None:
                meta = payload
            elif entry.get("type") == "turn_context":
                settings = (payload.get("collaboration_mode") or {}).get("settings") or {}
                model = payload.get("model") or settings.get("model")
                turns.append((payload.get("turn_id"), model, payload.get("effort") or settings.get("reasoning_effort")))
            elif payload.get("type") == "function_call" and payload.get("name") == "spawn_agent":
                calls.add(payload.get("call_id"))
            elif payload.get("type") == "function_call_output" and payload.get("call_id") in calls:
                output = payload.get("output")
                try:
                    task = json.loads(output) if isinstance(output, str) else output
                except ValueError:
                    task = None
                if isinstance(task, dict) and task.get("task_name"):
                    spawned.add(task["task_name"])
    return meta or {}, turns, spawned, whole


def agent_path(meta):
    source = meta.get("source")
    spawn = source.get("subagent") if isinstance(source, dict) else None
    return (spawn.get("thread_spawn") or {}).get("agent_path") if isinstance(spawn, dict) else None


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
        children.setdefault(meta.get("parent_thread_id"), []).append(session)
    cache = {}

    def load(session):
        if session not in cache:
            cache[session] = read(rollouts[session])
        return cache[session]

    def own_pairs(session):
        parent = metas[session].get("parent_thread_id")
        inherited = {turn[0] for turn in load(parent)[1]} if parent in rollouts else set()
        return [(model, effort) for turn_id, model, effort in load(session)[1] if turn_id not in inherited]

    failed = False
    for arg in args:
        if os.path.isfile(arg):
            label = os.path.basename(arg)
            text = pathlib.Path(arg).read_text(encoding="utf-8", errors="replace")
            header = re.search(r"^session id: (" + SESSION_ID.pattern + ")$", text, re.M)
            session = header.group(1) if header else None
        elif SESSION_ID.fullmatch(arg):
            label, session = arg, arg
        else:
            print(f"{os.path.basename(arg)}: neither a file nor a session id", file=sys.stderr)
            return 2
        if session is None or session not in rollouts:
            print(f"{label}: " + ("no `session id:` line" if session is None else f"session {session} has no rollout"))
            failed = True
            continue
        helpers, todo = [], list(children.get(session, []))
        while todo:
            helper = todo.pop()
            if helper != session and helper not in helpers:
                helpers.append(helper)
                todo += children.get(helper, [])
        meta, _, spawned, whole = load(session)
        pairs = list(dict.fromkeys(own_pairs(session)))
        helper_pairs = list(dict.fromkeys(pair for helper in helpers for pair in own_pairs(helper)))
        spawned = spawned.union(*(load(helper)[2] for helper in helpers))
        whole = whole and all(load(helper)[3] for helper in helpers)
        family = {session, metas[session].get("parent_thread_id"), *helpers}
        missing = spawned - {agent_path(metas[child]) for parent in family for child in children.get(parent, [])}
        started = re.sub(r"\.\d+Z$", "Z", str(meta.get("timestamp")))
        commit = (meta.get("git") or {}).get("commit_hash") or "unknown"
        line = f"{label}: {shown(pairs)}; session {session}, started {started}, codex {meta.get('cli_version')}, "
        line += f"commit {commit}"
        line += f"; {len(helpers)} helper sessions: {shown(helper_pairs)}" if helpers else "; no helper rollouts"
        line += f"; {len(missing)} spawned helpers have no rollout" if missing else ""
        line += "" if whole else "; a record does not parse"
        print(line)
        turns = pairs + helper_pairs
        failed |= bool(missing) or not whole or not turns or any(None in pair for pair in turns)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
