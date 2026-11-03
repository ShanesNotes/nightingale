---- MODULE step ----
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS End, Span, Budget, Ids, Moves, Slots, Cores, Guards, Design

NoGuard == 0
Intervals == {iv \in (0..End) \X (0..End) : iv[1] < iv[2]}
AllCores == [Intervals -> {"pass", "refuse"}]
PassingCore == {[iv \in Intervals |-> "pass"]}
ChartCores == {[iv \in Intervals |-> IF iv = refused THEN "refuse" ELSE "pass"] : refused \in {<<0, 2>>, <<0, 3>>}}
Max(a, b) == IF a > b THEN a ELSE b
Min(a, b) == IF a < b THEN a ELSE b
Largest(set) == CHOOSE x \in set : \A y \in set : y <= x

VARIABLES core, guard, fires, stamp, run, events, checkpoints, observed, last
vars == <<core, guard, fires, stamp, run, events, checkpoints, observed, last>>

NoRetry == [span |-> 0, n |-> 0, f |-> 0]
Image(r) == <<r.head, r.fence, r.applied, r.stream>>
Opening ==
  [head |-> 0, fence |-> 0, admitted |-> {}, applied |-> {}, stream |-> 0, phase |-> "running",
   retry |-> NoRetry, log |-> <<>>, stops |-> {}, restored |-> FALSE,
   trail |-> <<[image |-> <<0, 0, {}, 0>>, chunk |-> <<>>]>>]

Earliest(r) == Max(r.head, r.fence)
Waiting(r) == r.admitted \ r.applied
Due(r, s) == {i \in Waiting(r) : stamp[i] <= s}
NextWaiting(r) ==
  LET later == {stamp[i] : i \in {j \in Waiting(r) : stamp[j] > r.head}}
  IN IF later = {} THEN End + 1 ELSE CHOOSE x \in later : \A y \in later : x <= y
Crosses(r, e) == guard # NoGuard /\ r.stream = 0 /\ r.head < guard /\ guard <= e
Solve(t) == core[<<t.s, t.e>>]

RECURSIVE HalveBelow(_, _)
HalveBelow(x, len) == IF x < len THEN x ELSE HalveBelow(x \div 2, len)

Outcome(kind, t, stops, retry) == [kind |-> kind, t |-> t, fired |-> FALSE, stops |-> stops, retry |-> retry]
Refuse(t, stops) ==
  LET retry == HalveBelow(t.span \div 2, t.e - t.s)
  IN IF t.n >= Budget \/ retry < 1 THEN Outcome("faulted", t, stops, NoRetry)
     ELSE Outcome("refused", t, stops, [span |-> retry, n |-> t.n + 1, f |-> t.f])

RECURSIVE Bracket(_, _, _, _, _)
Bracket(r, t, stops, lo, hi) ==
  IF hi - lo <= 1 THEN Outcome("landed", [t EXCEPT !.e = t.s + hi], stops, NoRetry)
  ELSE LET trial == [t EXCEPT !.e = t.s + lo + (hi - lo) \div 2]
       IN IF Solve(trial) = "refuse" THEN Refuse(trial, stops)
          ELSE IF Crosses(r, trial.e) THEN Bracket(r, t, stops, lo, trial.e - t.s)
          ELSE Bracket(r, t, stops, trial.e - t.s, hi)

Solved(r, t, stops) ==
  IF Solve(t) = "refuse" THEN Refuse(t, stops)
  ELSE IF Crosses(r, t.e) THEN Bracket(r, t, stops, 0, t.e - t.s)
  ELSE Outcome("passed", t, stops, NoRetry)

Fire(out) ==
  CASE out.kind = "landed" ->
         IF fires THEN [out EXCEPT !.kind = "accepted", !.fired = TRUE] ELSE Refuse(out.t, out.stops)
    [] out.kind = "passed" -> [out EXCEPT !.kind = "accepted"]
    [] OTHER -> out

CutAt(t, c) == [t EXCEPT !.e = c, !.f = c]

MainAttempt(r, t) ==
  LET blind == Fire(Solved(r, t, {}))
      c == NextWaiting(r)
  IN IF blind.kind # "accepted" \/ c >= blind.t.e THEN blind
     ELSE Fire(Solved(r, CutAt(t, c), {c}))

ClipAtIssueAttempt(r, t) ==
  LET c == NextWaiting(r)
  IN IF c < t.e THEN Fire(Solved(r, CutAt(t, c), {c})) ELSE Fire(Solved(r, t, {}))

ClipFirstAttempt(r, t) ==
  LET out == ClipAtIssueAttempt(r, t)
  IN IF out.kind = "refused" THEN [out EXCEPT !.retry.f = Max(@, out.t.e)] ELSE out

CutThenLandAttempt(r, t) ==
  LET c == NextWaiting(r)
  IN IF Solve(t) = "refuse" THEN Refuse(t, {})
     ELSE IF c >= t.e THEN Fire(Solved(r, t, {}))
     ELSE Fire(Solved(r, CutAt(t, c), {c}))

FireAfterCutAttempt(r, t) ==
  LET blind == Solved(r, t, {})
      c == NextWaiting(r)
  IN IF blind.kind \notin {"passed", "landed"} THEN blind
     ELSE IF c >= blind.t.e THEN Fire(blind)
     ELSE Fire(Solved(r, CutAt(t, c), {c}))

Attempt(r, span, n, f) ==
  LET t == [s |-> r.head, e |-> Min(r.head + span, End), n |-> n, f |-> f, span |-> span]
  IN CASE Design = "clipAtIssue" -> ClipAtIssueAttempt(r, t)
       [] Design = "clipFirst" -> ClipFirstAttempt(r, t)
       [] Design = "cutThenLand" -> CutThenLandAttempt(r, t)
       [] Design = "fireAfterCut" -> FireAfterCutAttempt(r, t)
       [] OTHER -> MainAttempt(r, t)

Call(r) ==
  IF r.phase = "running" THEN Attempt(r, Span, 0, r.head)
  ELSE Attempt(r, r.retry.span, r.retry.n, r.retry.f)
CanCall(r) == r.phase = "retrying" \/ (r.phase = "running" /\ r.head < End)

Entry(r, out) ==
  [kind |-> out.kind, s |-> out.t.s, e |-> out.t.e, n |-> out.t.n, f |-> out.t.f,
   fired |-> out.fired, due |-> Due(r, out.t.s)]

Settle(r, out) ==
  LET logged == [r EXCEPT !.log = Append(@, Entry(r, out)), !.stops = @ \cup out.stops]
      next == [logged EXCEPT !.head = out.t.e, !.applied = @ \cup Due(r, out.t.s),
                             !.fence = Max(out.t.f, r.fence),
                             !.stream = @ + (IF out.fired THEN 1 ELSE 0),
                             !.phase = "running", !.retry = NoRetry]
  IN CASE out.kind = "accepted" ->
            [next EXCEPT !.trail = Append(@, [image |-> Image(next),
                                               chunk |-> <<out.t.s, out.t.e, Due(r, out.t.s) \cap Moves>>])]
       [] out.kind = "refused" -> [logged EXCEPT !.phase = "retrying", !.retry = out.retry]
       [] OTHER -> [logged EXCEPT !.phase = "faulted", !.retry = NoRetry]

RECURSIVE Calls(_, _)
Calls(r, n) == IF n = 0 \/ ~CanCall(r) THEN r ELSE Calls(Settle(r, Call(r)), n - 1)
KnownLog(admitted, n) == Calls([Opening EXCEPT !.admitted = admitted], n).log

Accepted(log) == SelectSeq(log, LAMBDA x : x.kind = "accepted")

Admits(r, i) == IF Design = "noFence" THEN stamp[i] >= r.head ELSE stamp[i] >= Earliest(r)

InsideLastStep(r, i) ==
  Design = "rollback" /\ Len(r.trail) > 1 /\ stamp[i] < Earliest(r)
  /\ stamp[i] > r.trail[Len(r.trail) - 1].image[1]

RolledBack(r) ==
  LET k == Len(r.trail) - 1
      accepted == {j \in 1..Len(r.log) : r.log[j].kind = "accepted"}
      kept == accepted \ {Largest(accepted)}
  IN [r EXCEPT !.head = r.trail[k].image[1], !.fence = r.trail[k].image[2],
               !.applied = r.trail[k].image[3], !.stream = r.trail[k].image[4],
               !.trail = SubSeq(@, 1, k), !.log = SubSeq(@, 1, IF kept = {} THEN 0 ELSE Largest(kept))]

Seal(trail) == IF Design = "ledgerRoot" THEN [k \in 1..Len(trail) |-> trail[k].chunk] ELSE trail
OnThisBranch(ck) ==
  /\ ck.taken
  /\ Len(ck.run.trail) <= Len(run.trail)
  /\ Seal(ck.run.trail) = Seal(SubSeq(run.trail, 1, Len(ck.run.trail)))

NoCheckpoint == [taken |-> FALSE, run |-> Opening, events |-> {}]

Init ==
  /\ core \in Cores
  /\ guard \in Guards
  /\ fires \in BOOLEAN
  /\ stamp \in [Ids -> 1..(End - 1)]
  /\ run = Opening
  /\ events = {}
  /\ checkpoints = [k \in Slots |-> NoCheckpoint]
  /\ observed = <<>>
  /\ last = [act |-> "admit", out |-> "", stamp |-> 0, before |-> Opening, beforeEvents |-> {}]

Fixed == UNCHANGED <<core, guard, fires, stamp>>
Did(act, out, s) == last' = [act |-> act, out |-> out, stamp |-> s, before |-> run, beforeEvents |-> events]

Step ==
  /\ CanCall(run)
  /\ LET out == Call(run) IN
     /\ run' = Settle(run, out)
     /\ observed' = IF out.kind = "accepted" THEN Append(observed, Entry(run, out)) ELSE observed
     /\ Did(IF run.phase = "running" THEN "step" ELSE "retry", out.kind, 0)
  /\ Fixed /\ UNCHANGED <<events, checkpoints>>

Send(i) ==
  /\ run.phase = "running"
  /\ i \notin run.admitted
  /\ CASE Admits(run, i) ->
            /\ run' = [run EXCEPT !.admitted = @ \cup {i}]
            /\ events' = events \cup {i}
            /\ Did("send", "admitted", stamp[i])
       [] InsideLastStep(run, i) ->
            /\ run' = [RolledBack(run) EXCEPT !.admitted = @ \cup {i}]
            /\ events' = events \cup {i}
            /\ Did("send", "admitted", stamp[i])
       [] OTHER ->
            /\ UNCHANGED <<run, events>>
            /\ Did("send", "refused", stamp[i])
  /\ Fixed /\ UNCHANGED <<checkpoints, observed>>

TakeCheckpoint(k) ==
  /\ run.phase = "running"
  /\ checkpoints' = [checkpoints EXCEPT ![k] = [taken |-> TRUE, run |-> run, events |-> events]]
  /\ Did("checkpoint", "", 0)
  /\ Fixed /\ UNCHANGED <<run, events, observed>>

Restore(k, act) ==
  /\ run' = [checkpoints[k].run EXCEPT !.restored = TRUE]
  /\ events' = checkpoints[k].events
  /\ observed' = Accepted(checkpoints[k].run.log)
  /\ Did(act, "", 0)
  /\ Fixed /\ UNCHANGED checkpoints

Resume(k) == checkpoints[k].taken /\ Restore(k, "resume")
Rewind(k) == run.phase = "running" /\ OnThisBranch(checkpoints[k]) /\ Restore(k, "rewind")
Recover(k) == run.phase = "faulted" /\ OnThisBranch(checkpoints[k]) /\ Restore(k, "recover")

Done ==
  /\ run.phase = "faulted" \/ (run.phase = "running" /\ run.head = End)
  /\ UNCHANGED vars

Next ==
  \/ Step
  \/ \E i \in Ids : Send(i)
  \/ \E k \in Slots : TakeCheckpoint(k) \/ Resume(k) \/ Rewind(k) \/ Recover(k)
  \/ Done

Spec == Init /\ [][Next]_vars

StateAndInputs(r) == <<r.head, r.fence, r.admitted, r.applied, r.stream, r.trail>>

RefusalChangesNothing ==
  last.act \in {"step", "retry"} /\ last.out \in {"refused", "faulted"} =>
    StateAndInputs(run) = StateAndInputs(last.before)

InputAppliesOnce ==
  /\ \A j, k \in 1..Len(run.log) :
       j < k /\ run.log[j].kind = "accepted" /\ run.log[k].kind = "accepted" =>
         run.log[j].due \cap run.log[k].due = {}
  /\ run.phase = "running" /\ run.head = End => run.admitted \subseteq run.applied

LateInputGivesKnownBits ==
  ~run.restored => run.log = KnownLog(events, Len(run.log))

BackDatedInputRefused ==
  last.act = "send" =>
    ((last.out = "refused") <=>
       (last.stamp < last.before.head \/ \E c \in last.before.stops : last.stamp < c))

RestoredRunGivesKnownBits ==
  run.restored => run.log = KnownLog(events, Len(run.log))

RewindLandsOnThisBranch ==
  last.act \in {"rewind", "recover"} => run.trail = SubSeq(last.before.trail, 1, Len(run.trail))

EventLogOnlyAppends ==
  last.act \in {"step", "retry", "send", "checkpoint"} => last.beforeEvents \subseteq events

PublishedStepsStay == Accepted(run.log) = observed
====
