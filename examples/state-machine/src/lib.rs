//! Testing a state machine with `test-better`.
//!
//! The classic turnstile: it is [`Locked`](State::Locked) until a coin goes
//! in, then [`Unlocked`](State::Unlocked) until someone pushes through. A
//! state machine is a transition function plus an enum, and `test-better`
//! tests it well: `matches_variant!` asserts on the resulting variant, and a
//! `fold` over a sequence of events checks a whole run in one assertion.
//!
//! Run the suite with `cargo test -p state-machine-example`.

/// The turnstile's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The arm is locked; a push does nothing.
    Locked,
    /// A coin has been paid; the next push goes through.
    Unlocked,
}

/// An event the turnstile can receive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A coin is inserted.
    Coin,
    /// Someone pushes the arm.
    Push,
}

/// The transition function: the next state, given the current state and an event.
///
/// - A `Coin` always unlocks (and a second coin is harmless).
/// - A `Push` from `Unlocked` lets one person through and re-locks.
/// - A `Push` from `Locked` is ignored.
#[must_use]
pub fn next(state: State, event: Event) -> State {
    match (state, event) {
        (_, Event::Coin) => State::Unlocked,
        (State::Unlocked, Event::Push) => State::Locked,
        (State::Locked, Event::Push) => State::Locked,
    }
}

/// Runs a sequence of events from a starting state and returns the final state.
#[must_use]
pub fn run(start: State, events: &[Event]) -> State {
    events
        .iter()
        .fold(start, |state, &event| next(state, event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_better::matches_variant;
    use test_better::prelude::*;

    #[test]
    fn a_coin_unlocks_a_locked_turnstile() -> TestResult {
        check!(next(State::Locked, Event::Coin)).satisfies(eq(State::Unlocked))
    }

    #[test]
    fn a_push_on_a_locked_turnstile_keeps_it_locked() -> TestResult {
        check!(next(State::Locked, Event::Push)).satisfies(eq(State::Locked))
    }

    #[test]
    fn a_push_on_an_unlocked_turnstile_relocks_it() -> TestResult {
        check!(next(State::Unlocked, Event::Push)).satisfies(eq(State::Locked))
    }

    #[test]
    fn coin_then_push_lets_exactly_one_person_through() -> TestResult {
        // Pay, walk through, and the arm is locked again.
        let end = run(State::Locked, &[Event::Coin, Event::Push]);
        // `matches_variant!` asserts on the variant; here it carries no fields,
        // so the match is the whole assertion.
        check!(end).satisfies(matches_variant!(State::Locked))
    }

    #[test]
    fn a_second_push_without_paying_does_not_get_through() -> TestResult {
        // One coin, two pushes: the second push finds the arm already locked.
        let end = run(State::Locked, &[Event::Coin, Event::Push, Event::Push]);
        check!(end).satisfies(eq(State::Locked))
    }

    #[test]
    fn a_second_coin_before_pushing_is_harmless() -> TestResult {
        let end = run(State::Locked, &[Event::Coin, Event::Coin]);
        check!(end).satisfies(eq(State::Unlocked))
    }

    #[test]
    fn an_empty_event_sequence_leaves_the_state_untouched() -> TestResult {
        check!(run(State::Unlocked, &[])).satisfies(eq(State::Unlocked))
    }
}
