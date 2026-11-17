# 36: The whole body's filtration and output answer posture the wrong way

Type: task
Status: open
Blocked by: none

**Why.** The slice's posture checks (`crates/ng-validate/tests/arena_body.rs`) had not been run on the whole body. [Ticket 09](09-tissues-respire.md)'s earn run ran them, from one admitted start, with means over 30 to 40 s. Standing, filtration rises to 2.0248 from 1.9979 flat, where the check asks it to fall by at most 5%. Seated, output rises to 81.13 from 78.10 flat while stroke volume falls, where the check asks both to fall, and filtration rises to 2.0411. Every other check passes in both postures: the pressure stands above supine (105.5 standing and 107.1 seated against 99.9), the rate rises, sympathetic traffic rises and stays under 0.8 (0.53 and 0.49), and nothing hunts (swing under 0.02). It was so before arrow 78 was wired, with the heart's work still paid at the receipt.

- [ ] Which held controller or missing law makes each answer, by a run with each candidate restored
- [ ] The posture checks run on the whole body as a test, and each check that cannot yet pass is named beside the ticket that will make it pass
