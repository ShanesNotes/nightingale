---- MODULE system ----
EXTENDS Naturals

VARIABLES build, run, parity, waiting, unread, entries, scored, act, before

vars == <<build, run, parity, waiting, unread, entries, scored, act, before>>

Body == [run |-> run, parity |-> parity, waiting |-> waiting]

Did(a) == act' = a /\ before' = Body

Init ==
  /\ build = "declared" /\ run = "none"
  /\ parity = 0 /\ waiting = FALSE /\ unread = FALSE
  /\ entries = FALSE /\ scored = FALSE
  /\ act = "init" /\ before = Body

Built(from, to) ==
  /\ build = from /\ build' = to
  /\ Did(to)
  /\ UNCHANGED <<run, parity, waiting, unread, entries, scored>>

\* ng-contract anatomy.rs and scope.rs: AnatomyBuilder::finish checks the map, assemble compiles the laws.
Compile == Built("declared", "compiled")
\* ng-contract packet.rs: CompiledBody::bind parses the packet against the parameter layout.
Bind == Built("compiled", "bound")
Refuse(from) == Built(from, "notAdmitted")

\* ng-contract admission.rs and ng-validate initialize.rs: the one admission solve, then Session::admit.
Admit ==
  /\ build = "bound" /\ build' = "admitted" /\ run' = "accepted"
  /\ Did("admit")
  /\ UNCHANGED <<parity, waiting, unread, entries, scored>>

\* ng-record session.rs: Session::enqueue takes only an AuthorizedInput built from a command's receipt.
Command ==
  /\ run = "accepted" /\ ~waiting /\ waiting' = TRUE
  /\ Did("command")
  /\ UNCHANGED <<build, run, parity, unread, entries, scored>>

\* ng-record session.rs: Session::step and Session<Retrying>::retry, through PatientRecord::publish.
Publish(from) ==
  /\ run = from /\ run' = "accepted"
  /\ parity' = 1 - parity
  /\ waiting' = FALSE /\ unread' = TRUE
  /\ Did("accept")
  /\ UNCHANGED <<build, entries, scored>>

Moves(from, to, a) ==
  /\ run = from /\ run' = to
  /\ Did(a)
  /\ UNCHANGED <<build, parity, waiting, unread, entries, scored>>

StepRefused == Moves("accepted", "retrying", "refuse")
NoShorterInterval == Moves("retrying", "faulted", "exhausted")
EngineFault == Moves("accepted", "faulted", "fault")

\* ng-record session.rs: Session<Faulted>::recover; the observer restores from its checkpoint beside it.
Recover ==
  /\ run = "faulted" /\ run' = "accepted"
  /\ parity' = 0 /\ unread' = FALSE
  /\ Did("recover")
  /\ UNCHANGED <<build, waiting, entries, scored>>

\* ng-observe observer.rs: Observer::advance reads the next accepted revision and writes nothing back.
Observe ==
  /\ unread /\ unread' = FALSE
  /\ Did("observe")
  /\ UNCHANGED <<build, run, parity, waiting, entries, scored>>

Document ==
  /\ ~entries /\ entries' = TRUE
  /\ Did("document")
  /\ UNCHANGED <<build, run, parity, waiting, unread, scored>>

\* ng-validate arena.rs: Arena::score builds its own body, runs its own sessions and keeps each part's result.
Score ==
  /\ ~scored /\ scored' = TRUE
  /\ Did("score")
  /\ UNCHANGED <<build, run, parity, waiting, unread, entries>>

Done ==
  /\ build = "notAdmitted"
  /\ UNCHANGED vars

Next ==
  \/ Compile \/ Refuse("declared")
  \/ Bind \/ Refuse("compiled")
  \/ Admit \/ Refuse("bound")
  \/ Command \/ Publish("accepted") \/ Publish("retrying")
  \/ StepRefused \/ NoShorterInterval \/ EngineFault \/ Recover
  \/ Observe \/ Document \/ Score
  \/ Done

Spec == Init /\ [][Next]_vars

NoStepBeforeAdmission ==
  run # "none" => build = "admitted"

AdmissionSolvesOnce ==
  act = "admit" => before.run = "none"

ReadersNeverWritePhysiology ==
  act \in {"observe", "document", "score"} => Body = before

OnlyACommandEntersTheBody ==
  (waiting /\ ~before.waiting) => act = "command"

OnlyAnAcceptedStepAppliesAnInput ==
  (before.waiting /\ ~waiting) => act = "accept"

OnlyTheRecordCrateChangesState ==
  parity # before.parity => act \in {"accept", "recover"}

ObserversReadOnlyAcceptedSteps ==
  unread => run # "none"

AFaultTakesNoStep ==
  act \in {"accept", "refuse", "command"} => before.run \in {"accepted", "retrying"}
====
