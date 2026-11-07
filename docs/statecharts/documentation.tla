---- MODULE documentation ----
EXTENDS Naturals

\* The chart is not built. Every phase here is a placeholder; the rule it holds binds today (SRS rules 1 and 13).
CONSTANTS MaxEntries, MaxCommands

VARIABLES phase, entries, waiting, applied, act, before

vars == <<phase, entries, waiting, applied, act, before>>

Body == <<waiting, applied>>

Did(a) == act' = a /\ before' = Body

Init ==
  /\ phase = "open" /\ entries = 0 /\ waiting = 0 /\ applied = 0
  /\ act = "open" /\ before = <<0, 0>>

Write(a) ==
  /\ phase = "open" /\ entries < MaxEntries
  /\ phase' = "entry" /\ entries' = entries + 1
  /\ Did(a)
  /\ UNCHANGED <<waiting, applied>>

Note == Write("note")
ChartReading == Write("reading")

Kept ==
  /\ phase = "entry" /\ phase' = "open"
  /\ Did("kept")
  /\ UNCHANGED <<entries, waiting, applied>>

Order ==
  /\ phase = "open" /\ waiting + applied < MaxCommands
  /\ phase' = "order"
  /\ Did("order")
  /\ UNCHANGED <<entries, waiting, applied>>

Unauthorized ==
  /\ phase = "order" /\ phase' = "open"
  /\ Did("unauthorized")
  /\ UNCHANGED <<entries, waiting, applied>>

\* ng-record record.rs: AuthorizedInput::from_command_receipt is the only input Session::enqueue takes.
Command ==
  /\ phase = "order" /\ phase' = "command"
  /\ waiting' = waiting + 1
  /\ Did("command")
  /\ UNCHANGED <<entries, applied>>

Applied ==
  /\ phase = "command" /\ phase' = "open"
  /\ waiting' = waiting - 1 /\ applied' = applied + 1
  /\ Did("applied")
  /\ UNCHANGED entries

Done ==
  /\ phase = "open" /\ entries = MaxEntries /\ waiting + applied = MaxCommands
  /\ UNCHANGED vars

Next == Note \/ ChartReading \/ Kept \/ Order \/ Unauthorized \/ Command \/ Applied \/ Done

Spec == Init /\ [][Next]_vars

AChartEntryIsNeverState ==
  act \in {"note", "reading", "kept", "order", "unauthorized"} => Body = before

OnlyAnAuthorizedCommandReachesTheBody ==
  waiting > before[1] => act = "command"

TheBodyAppliesOnlyWhatACommandEnqueued ==
  applied > before[2] => (act = "applied" /\ before[1] > 0)
====
