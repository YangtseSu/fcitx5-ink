// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Recognition request generations. Recognition runs off the GTK thread, so a
//! result is only applied when it belongs to the latest request; edits made
//! while a request is in flight schedule one more run with the final strokes.

#[derive(Debug, Default)]
pub struct RecognitionState {
    generation: u64,
    in_flight: Option<u64>,
    pending: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Start {
    Start(u64),
    AlreadyRunning,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Finish {
    Apply,
    Restart(u64),
    Ignore,
}

impl RecognitionState {
    /// Register a recognition request for the current strokes.
    pub fn request(&mut self) -> Start {
        self.generation += 1;
        if self.in_flight.is_some() {
            self.pending = true;
            return Start::AlreadyRunning;
        }
        self.in_flight = Some(self.generation);
        Start::Start(self.generation)
    }

    /// Drop the pending request. An in-flight result becomes stale.
    pub fn cancel(&mut self) {
        self.generation += 1;
        self.pending = false;
    }

    /// Accept, restart, or ignore a completed worker generation.
    pub fn finish(&mut self, generation: u64) -> Finish {
        if self.in_flight != Some(generation) {
            return Finish::Ignore;
        }
        self.in_flight = None;
        if self.pending {
            self.pending = false;
            let next = self.generation;
            self.in_flight = Some(next);
            return Finish::Restart(next);
        }
        if generation == self.generation {
            Finish::Apply
        } else {
            Finish::Ignore
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_result_is_applied() {
        let mut state = RecognitionState::default();
        assert_eq!(state.request(), Start::Start(1));
        assert_eq!(state.finish(1), Finish::Apply);
    }

    #[test]
    fn a_request_during_recognition_restarts_with_the_latest_generation() {
        let mut state = RecognitionState::default();
        assert_eq!(state.request(), Start::Start(1));
        assert_eq!(state.request(), Start::AlreadyRunning);
        assert_eq!(state.request(), Start::AlreadyRunning);
        assert_eq!(state.finish(1), Finish::Restart(3));
        assert_eq!(state.finish(3), Finish::Apply);
    }

    #[test]
    fn a_cancelled_result_is_ignored() {
        let mut state = RecognitionState::default();
        assert_eq!(state.request(), Start::Start(1));
        state.cancel();
        assert_eq!(state.finish(1), Finish::Ignore);
    }

    #[test]
    fn a_request_after_cancel_restarts_once_the_old_worker_finishes() {
        let mut state = RecognitionState::default();
        assert_eq!(state.request(), Start::Start(1));
        state.cancel();
        assert_eq!(state.request(), Start::AlreadyRunning);
        assert_eq!(state.finish(1), Finish::Restart(3));
        assert_eq!(state.finish(3), Finish::Apply);
    }

    #[test]
    fn an_unknown_generation_is_ignored() {
        let mut state = RecognitionState::default();
        assert_eq!(state.finish(7), Finish::Ignore);
        assert_eq!(state.request(), Start::Start(1));
        assert_eq!(state.finish(7), Finish::Ignore);
        assert_eq!(state.finish(1), Finish::Apply);
    }
}
