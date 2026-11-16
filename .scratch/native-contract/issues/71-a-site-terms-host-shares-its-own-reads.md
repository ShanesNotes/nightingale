# 71: A site term's host shares its own reads

Spec: `crates/ng-contract/src/law.rs` (`Written::shared`)

Type: task

Status: open

Blocked by: none

On main a law that takes a site term has its own identical reads merged by `Written::shared`, so the order its slopes are summed in changes. A test built to cancel shows slopes of zero where the inline form gives the law's own, with residuals equal bit for bit. A written-in term should change nothing of its host but its own contribution. Restricting the sharing to the term's own ops could move the slice's pins, so this ticket measures that first.

- [ ] Measure whether restricting `Written::shared` to the term's own ops moves the slice's or the whole body's pins
- [ ] The change, with the cancelling test
