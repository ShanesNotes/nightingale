"""Prints the model and reasoning effort each Codex pass ran with, read from its rollout.

    python3 -I tools/codex-provenance.py PASS [PASS ...]

Python 3.11 stdlib only. A PASS is a Codex run's saved output, whose header names its `session id:`, or that id.
Its rollout is the file under $CODEX_HOME/sessions, default ~/.codex/sessions, whose name ends in the id. Each pass
prints every model and effort its turns ran with, its session, start, Codex version and the commit Codex saw at the
start, then the models and efforts of the helper sessions it spawned. `codex review` runs its turns in a helper,
so its own session shows no turn. Nothing else of a rollout prints: no prompt, path or account.

Exit 0 when every pass's session or its helpers record a turn, and every turn records a model and an effort;
1 otherwise; 2 usage.
"""
import json
import os
import pathlib
import re
import sys

SESSION_ID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
SESSIONS = pathlib.Path(os.environ.get("CODEX_HOME") or pathlib.Path.home() / ".codex") / "sessions"


def records(path):
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            try:
                entry = json.loads(line)
            except ValueError:
                continue
            if isinstance(entry, dict) and isinstance(entry.get("payload"), dict):
                yield entry.get("type"), entry["payload"]


def session_meta(path):
    return next((payload for kind, payload in records(path) if kind == "session_meta"), {})


def turns(path):
    out = []
    for kind, payload in records(path):
        if kind == "turn_context":
            settings = (payload.get("collaboration_mode") or {}).get("settings") or {}
            model = payload.get("model") or settings.get("model")
            pair = (model, payload.get("effort") or settings.get("reasoning_effort"))
            if pair not in out:
                out.append(pair)
    return out


def shown(pairs):
    return " and ".join(f"{model or '?'} {effort or '?'}" for model, effort in pairs) or "no turn"


def main(args):
    if not args:
        print(__doc__, file=sys.stderr)
        return 2
    rollouts = {p.name[-42:-6]: p for p in sorted(SESSIONS.rglob("rollout-*.jsonl"))}
    children = {}
    for session, path in rollouts.items():
        children.setdefault(session_meta(path).get("parent_thread_id"), []).append(session)
    failed = False
    for arg in args:
        if os.path.isfile(arg):
            label = os.path.basename(arg)
            text = pathlib.Path(arg).read_text(encoding="utf-8", errors="replace")
            sessions = list(dict.fromkeys(re.findall(r"session id: (" + SESSION_ID.pattern + ")", text)))
        elif SESSION_ID.fullmatch(arg):
            label, sessions = arg, [arg]
        else:
            print(f"{arg}: neither a file nor a session id", file=sys.stderr)
            return 2
        if not sessions:
            print(f"{label}: no `session id:` line")
            failed = True
        for session in sessions:
            if session not in rollouts:
                print(f"{label}: session {session} has no rollout under the Codex sessions")
                failed = True
                continue
            meta, pairs = session_meta(rollouts[session]), turns(rollouts[session])
            helpers, todo = [], list(children.get(session, []))
            while todo:
                helper = todo.pop()
                if helper != session and helper not in helpers:
                    helpers.append(helper)
                    todo += children.get(helper, [])
            helper_pairs = list(dict.fromkeys(p for h in helpers for p in turns(rollouts[h])))
            started = re.sub(r"\.\d+Z$", "Z", str(meta.get("timestamp")))
            commit = (meta.get("git") or {}).get("commit_hash") or "unknown"
            line = f"{label}: {shown(pairs)}; session {session}, started {started}, codex {meta.get('cli_version')}, "
            line += f"commit {commit}"
            line += f"; {len(helpers)} helper sessions: {shown(helper_pairs)}" if helpers else "; no helper sessions"
            print(line)
            failed |= not (pairs + helper_pairs) or any(None in p for p in pairs + helper_pairs)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
