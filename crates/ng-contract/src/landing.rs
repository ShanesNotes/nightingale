#[derive(Clone, Copy, PartialEq)]
enum End {
    Bottom,
    Top,
}

/// The search's only state: a verified length that does not cross and one that does. Plain arithmetic on
/// values `verify` produced, so its tickets are a function of the sealed past, the span and the refusals of
/// this base, whatever waits.
pub(crate) struct Bracket {
    bottom_ns: u64,
    bottom: f64,
    top_ns: u64,
    top: f64,
    tolerance_ns: u64,
    moved: End,
    /// A first trial near the root rarely halves a bracket, and calling that a stall sends the second trial to
    /// the middle, far from the root and into a longer ticket's refusals.
    earlier_ns: u64,
    stalled: bool,
}
impl Bracket {
    /// `base` is the gap at the accepted state, the bottom of length zero.
    pub(crate) fn new(base: f64, top_ns: u64, top: f64, tolerance_ns: u64) -> Self {
        Self {
            bottom_ns: 0,
            bottom: base,
            top_ns,
            top,
            tolerance_ns,
            moved: End::Top,
            earlier_ns: u64::MAX,
            stalled: false,
        }
    }
    /// A landing: a ticket that crosses, and one at most the tolerance shorter that does not.
    pub(crate) fn closed(&self) -> bool {
        self.top_ns - self.bottom_ns <= self.tolerance_ns
    }
    /// The next trial's length, strictly inside the bracket. The secant's root, half a tolerance toward the end
    /// that has not moved, so two trials close a straight gap. Inside two tolerances the middle closes on
    /// either outcome. Past that, a trial stays a tolerance from each end, so it closes the bracket or moves
    /// an end by a tolerance, and no ticket is shorter than the tolerance for a root near the head.
    pub(crate) fn aim(&self) -> u64 {
        let Self {
            bottom_ns,
            bottom,
            top_ns,
            top,
            tolerance_ns,
            moved,
            stalled,
            ..
        } = *self;
        let width = top_ns - bottom_ns;
        let slope = top - bottom;
        if stalled || width / 2 <= tolerance_ns || !(slope > 0.0 && slope.is_finite()) {
            return bottom_ns + width / 2;
        }
        let root = bottom_ns as f64 + width as f64 * -bottom / slope;
        let half = tolerance_ns as f64 / 2.0;
        let aim = if moved == End::Top {
            root - half
        } else {
            root + half
        };
        (aim as u64).clamp(bottom_ns + tolerance_ns, top_ns - tolerance_ns)
    }
    /// Two trials that together failed to halve the bracket make the next one bisect, so its width halves at
    /// least every three trials, and a span below 2^64 ns takes at most 192 trials at any tolerance.
    pub(crate) fn note(&mut self, length_ns: u64, gap: f64, crossed: bool) {
        let width = self.top_ns - self.bottom_ns;
        if crossed {
            self.top_ns = length_ns;
            self.top = gap;
            self.moved = End::Top;
        } else {
            self.bottom_ns = length_ns;
            self.bottom = gap;
            self.moved = End::Bottom;
        }
        self.stalled = self.top_ns - self.bottom_ns > self.earlier_ns / 2;
        self.earlier_ns = width;
    }
}

#[cfg(test)]
mod tests {
    use super::Bracket;

    fn trials(base: f64, top_ns: u64, top: f64, tolerance_ns: u64, mut gap: impl FnMut(u64) -> f64) -> u32 {
        let mut bracket = Bracket::new(base, top_ns, top, tolerance_ns);
        let mut spent = 0;
        while !bracket.closed() {
            let length = bracket.aim();
            assert!(bracket.bottom_ns < length && length < bracket.top_ns);
            let value = gap(length);
            bracket.note(length, value, value >= 0.0);
            spent += 1;
        }
        spent
    }

    #[test]
    fn a_ticket_no_longer_than_the_tolerance_has_landed_with_no_trial() {
        assert_eq!(trials(-1.0, 1_000, 1.0, 1_000, |_| panic!("no trial")), 0);
    }

    #[test]
    fn a_straight_gap_closes_in_two_trials() {
        let span = 8_000_000_000_u64;
        let straight = |length: u64| length as f64 / 3_000_000_000.0 - 1.0;
        assert_eq!(trials(-1.0, span, straight(span), 1_000_000, straight), 2);
    }

    #[test]
    fn a_root_near_the_head_takes_no_ticket_shorter_than_the_tolerance() {
        let span = 8_000_000_000_u64;
        let tolerance = 1_000_000;
        let near = |length: u64| length as f64 / 200_000.0 - 1.0;
        let mut shortest = u64::MAX;
        trials(-1.0, span, near(span), tolerance, |length| {
            shortest = shortest.min(length);
            near(length)
        });
        assert_eq!(shortest, tolerance);
    }

    #[test]
    fn a_base_already_past_its_draw_lands_at_the_first_trial() {
        assert_eq!(trials(5.0, 1 << 40, 7.0, 1_000, |length| 5.0 + length as f64), 1);
    }

    #[test]
    fn a_jagged_gap_over_the_whole_range_closes_inside_the_bound() {
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut jagged = move |_: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 * if state & 1 == 0 { 1.0 } else { -1.0 }
        };
        for _ in 0..1_000 {
            let spent = trials(-1.0, u64::MAX, 1.0, 1, &mut jagged);
            assert!(spent <= 192, "{spent} trials");
        }
    }

    #[test]
    fn a_gap_that_is_no_number_closes_and_panics_nothing() {
        assert!(trials(-1.0, 1 << 40, f64::INFINITY, 1, |_| f64::NAN) <= 192);
        assert!(trials(f64::NAN, 1 << 40, f64::NAN, 1, |_| 1.0) <= 192);
        assert!(trials(5.0, 1 << 40, 5.0, 1_000, |_| 5.0) <= 192);
    }
}
