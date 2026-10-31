---- MODULE engine ----
EXTENDS Naturals

CONSTANTS MaxRevision, MaxInterval, MinInterval, MaxBranch

Phases == {"mapDeclared", "mapChecked", "compiled", "bound",
           "accepted", "proposed", "refused", "faulted",
           "notAdmitted"}

VARIABLES phase, mapOk, bodyOk, packetOk, startOk,
          revision, base, interval, branch,
          act, prevPhase, prevRevision, prevBranch, prevInterval

vars == <<phase, mapOk, bodyOk, packetOk, startOk, revision, base, interval,
          branch, act, prevPhase, prevRevision, prevBranch, prevInterval>>

Remember(a) ==
  /\ act' = a
  /\ prevPhase' = phase
  /\ prevRevision' = revision
  /\ prevBranch' = branch
  /\ prevInterval' = interval

Init ==
  /\ phase = "mapDeclared"
  /\ mapOk = FALSE /\ bodyOk = FALSE /\ packetOk = FALSE /\ startOk = FALSE
  /\ revision = 0 /\ base = 0 /\ interval = MaxInterval /\ branch = 0
  /\ act = "init" /\ prevPhase = "mapDeclared"
  /\ prevRevision = 0 /\ prevBranch = 0 /\ prevInterval = MaxInterval

Fail(from) ==
  /\ phase = from
  /\ phase' = "notAdmitted"
  /\ Remember("fail")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, startOk, revision, base, interval, branch>>

\* crates/ng-contract/src/anatomy.rs: AnatomyBuilder::finish checks the map, Anatomy::digest hashes it.
MapPasses ==
  /\ phase = "mapDeclared" /\ phase' = "mapChecked" /\ mapOk' = TRUE
  /\ Remember("mapPasses")
  /\ UNCHANGED <<bodyOk, packetOk, startOk, revision, base, interval, branch>>
MapFails == Fail("mapDeclared")

\* Sketch ng_contract/scope.rs: Assembly::compile, refusing a missing or doubly owned law (native-contract 03).
LawsCompile ==
  /\ phase = "mapChecked" /\ phase' = "compiled" /\ bodyOk' = TRUE
  /\ Remember("compile")
  /\ UNCHANGED <<mapOk, packetOk, startOk, revision, base, interval, branch>>
CompileFails == Fail("mapChecked")

\* Sketch ng_contract/body.rs: CompiledBody::bind parses a packet against the parameter layout.
PacketBinds ==
  /\ phase = "compiled" /\ phase' = "bound" /\ packetOk' = TRUE
  /\ Remember("bind")
  /\ UNCHANGED <<mapOk, bodyOk, startOk, revision, base, interval, branch>>
BindFails == Fail("compiled")

\* Sketch ng_contract/admission.rs and ng_record.rs: the admission solve, then Session::admit (native-contract 08).
Admit ==
  /\ phase = "bound" /\ phase' = "accepted" /\ startOk' = TRUE
  /\ Remember("admit")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, revision, base, interval, branch>>
AdmitFails == Fail("bound")

\* SRS section 2 Step and sketch ng_record.rs Session::step, Step and retry (native-contract 04).
Propose ==
  /\ phase = "accepted" /\ phase' = "proposed"
  /\ base' = revision /\ interval' = MaxInterval
  /\ Remember("propose")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, startOk, revision, branch>>

Accept ==
  /\ phase = "proposed" /\ phase' = "accepted"
  /\ revision' = base + 1
  /\ Remember("accept")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, startOk, base, interval, branch>>

Refuse ==
  /\ phase = "proposed" /\ phase' = "refused"
  /\ Remember("refuse")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, startOk, revision, base, interval, branch>>

Retry ==
  /\ phase = "refused" /\ interval > MinInterval
  /\ phase' = "proposed" /\ interval' = interval - 1
  /\ Remember("retry")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, startOk, revision, base, branch>>

NoShorterInterval ==
  /\ phase = "refused" /\ interval = MinInterval
  /\ phase' = "faulted"
  /\ Remember("exhausted")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, startOk, revision, base, interval, branch>>

EngineFault ==
  /\ phase = "proposed" /\ phase' = "faulted"
  /\ Remember("fault")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, startOk, revision, base, interval, branch>>

\* Sketch ng_record.rs: Session<Faulted> has no step; recover opens a new branch from an earlier checkpoint.
Recover ==
  /\ phase = "faulted" /\ phase' = "accepted"
  /\ \E r \in 0..revision : revision' = r
  /\ branch' = branch + 1
  /\ Remember("recover")
  /\ UNCHANGED <<mapOk, bodyOk, packetOk, startOk, base, interval>>

Done ==
  /\ phase = "notAdmitted"
  /\ UNCHANGED vars

Next ==
  \/ MapPasses \/ MapFails
  \/ LawsCompile \/ CompileFails
  \/ PacketBinds \/ BindFails
  \/ Admit \/ AdmitFails
  \/ Propose \/ Accept \/ Refuse \/ Retry \/ NoShorterInterval \/ EngineFault
  \/ Recover \/ Done

Spec == Init /\ [][Next]_vars

Bound == revision <= MaxRevision /\ branch <= MaxBranch

RunPhases == {"accepted", "proposed", "refused", "faulted"}

TypeOK ==
  /\ phase \in Phases
  /\ act \in {"init", "fail", "mapPasses", "compile", "bind", "admit", "propose",
              "accept", "refuse", "retry", "exhausted", "fault", "recover"}
  /\ interval \in MinInterval..MaxInterval

NoStepBeforeAdmission ==
  phase \in RunPhases => (mapOk /\ bodyOk /\ packetOk /\ startOk)

NotAdmittedHasNoRun ==
  phase = "notAdmitted" => ~startOk

RefusalChangesNothing ==
  act = "refuse" => (revision = prevRevision /\ revision = base)

RetryIsShorterNeverBelowMinimum ==
  act = "retry" => (interval < prevInterval /\ interval >= MinInterval)

OnlyAcceptAdvancesState ==
  act \notin {"accept", "recover"} => revision = prevRevision

StepStartsFromAcceptedState ==
  act = "propose" => prevPhase = "accepted"

FaultIsLatched ==
  act \in {"propose", "accept", "retry"} => prevPhase # "faulted"

RecoverOpensNewBranch ==
  act = "recover" => (prevPhase = "faulted" /\ branch = prevBranch + 1 /\ revision <= prevRevision)
====
