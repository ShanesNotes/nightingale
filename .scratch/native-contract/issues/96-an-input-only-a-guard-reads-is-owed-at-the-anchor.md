# 96: An input only a guard reads is owed at the anchor

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 6, [body ticket 05](../../body/issues/05-the-days-inputs.md)'s rule for lasting inputs, and `crates/ng-contract/src/body.rs` (`CompiledBody::seal`)

Type: task

Status: open

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) runs a day whose inputs reach the body as lasting inputs, and a body that reads one with no value at the anchor moment must be refused at admission, not run on an unstated value. Body ticket 05 states the rule, and section 6 says admission refuses such a body with `InputUnstated`.

**What the break-it pass on nightingale#214 found.** A lasting input that only a guard reads is not in the list admission checks. The compiler keeps a guard's input reads with the guards, and `CompiledBody::seal` collects the lasting reads from the law instances only (`layout.instances`). Admission checks the anchor against that list, so it returns `Ok` where ticket 05 requires `InputUnstated`. Main behaves so: the test below fails there with `left: None` and `right: Some(Admission(InputUnstated { input: Sleep }))`.

**Live or waiting.** By reading main: no guard reads a lasting input today. Outside tests the one law that reads one is the chemoreflex's scale in `crates/ng-nervous/src/chemoreflex.rs`, which reads sleep and is no guard. Every other read of a lasting input in the crates is in a test. So the hole is waiting. No body ticket plans a guard on a lasting input: the bladder's void in [body ticket 22](../../body/issues/22-the-bladder.md) is a gate a law multiplies by, not a threshold, so this ticket blocks none.

- [ ] The test below is red before the change and green after it, in `crates/ng-contract/tests/solved_modes.rs`
- [ ] A body whose guards read no lasting input keeps its bytes: its model hash, its admitted seals and its pins, or the PR moves them once with the reason
- [ ] A mutant row that collects the lasting reads from the instances alone, CAUGHT by the test

The test, from the break-it pass on nightingale#214, as a patch to `crates/ng-contract/tests/solved_modes.rs`:

```diff
diff --git a/crates/ng-contract/tests/solved_modes.rs b/crates/ng-contract/tests/solved_modes.rs
index d0f7e62..4712732 100644
--- a/crates/ng-contract/tests/solved_modes.rs
+++ b/crates/ng-contract/tests/solved_modes.rs
@@ -16,6 +16,10 @@ fn reading() -> RowKind {
 }
 
 fn body() -> Result<CompiledBody, AdmissionError> {
+    switching_body(false)
+}
+
+fn switching_body(sleep_scales_return_threshold: bool) -> Result<CompiledBody, AdmissionError> {
     let mut map = AnatomyBuilder::new();
     let origin = Position::new(measure(0.0), measure(0.0), measure(0.0));
     let site = map.site::<TissueSite>("body", origin).id();
@@ -46,6 +50,12 @@ fn body() -> Result<CompiledBody, AdmissionError> {
         ns.event(Guard::new(entering, Crossing::Rising, a.to(&b)?));
         let leaving = ns.kernel(|mut k| {
             let (zero, value, threshold) = (k.zero(), k.state(&x), k.parameter(&threshold));
+            let threshold = if sleep_scales_return_threshold {
+                let asleep = k.asleep();
+                k.scale(threshold, asleep)
+            } else {
+                threshold
+            };
             let negative = k.subtract(zero, value);
             let gap = k.subtract(negative, threshold);
             k.finish(gap, b.only(), class)
@@ -279,3 +289,14 @@ fn a_guard_that_reads_no_value_names_its_mode() -> Result<(), InitializationFail
     );
     Ok(())
 }
+
+/// Body 05: "A body that reads one with no value at the anchor moment is refused (`InputUnstated`)."
+#[test]
+fn admission_refuses_unstated_sleep_read_only_by_a_guard() -> Result<(), InitializationFailure> {
+    let body = switching_body(true)?;
+    let expected = AdmissionError::InputUnstated {
+        input: Lasting::Sleep,
+    };
+    assert_eq!(admitted(&body, BASE).err(), Some(expected.into()));
+    Ok(())
+}
```

Sleep scales the threshold of the guard from mode `b` back to `a`. The packet starts in `a` with `x = 0` and the threshold for entering `b` at 1, so the return guard stays disarmed through admission, and no law instance reads sleep.

**Non-goals:** which lasting inputs a body reads, and the rule that an input scales a gain or a set point (ticket 05's review by reading).
