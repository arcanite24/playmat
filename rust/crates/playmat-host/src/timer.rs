//! The match's priority timer, as Java's `PriorityTimer` runs it (`GameController`'s `RESUME_TIMER` and
//! `PAUSE_TIMER`): each player has the table's time for the whole game, which counts down only while they are
//! asked something (the human) or think at priority (the AI). Each time a player's clock starts, they get the
//! table's buffer time again, which counts down first. A player whose time runs out loses.

use playmat_engine::state::Seat;
use std::time::{Duration, Instant};

/// The table's timer settings (`MatchTimeLimit`, `MatchBufferTime`); 0 seconds: no timer
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timer {
    pub priority_secs: u32,
    pub buffer_secs: u32,
    /// each seat's time left from the match's earlier games (`MatchImpl.initGame`: "2+ games must keep times");
    /// none: the whole priority time
    pub left: [Option<u32>; 2],
}

/// Java's `priorityTimeLeft` for a game without a timer
pub const NO_TIME_LIMIT: i64 = i32::MAX as i64;

/// Both players' clocks
#[derive(Clone, Debug)]
pub struct Clock {
    timer: Timer,
    left: [Duration; 2],
    buffer: [Duration; 2],
    /// the seat whose clock runs, since when
    running: Option<(Seat, Instant)>,
}

impl Clock {
    pub fn new(timer: Timer) -> Clock {
        let left = |seat: usize| Duration::from_secs(timer.left[seat].unwrap_or(timer.priority_secs).into());
        Clock { timer, left: [left(0), left(1)], buffer: [Duration::ZERO; 2], running: None }
    }

    pub fn on(&self) -> bool {
        self.timer.priority_secs > 0
    }

    pub fn timer(&self) -> Timer {
        self.timer
    }

    /// The seat's clock starts (and any other stops), with its buffer full again
    pub fn resume(&mut self, seat: Seat, now: Instant) {
        if !self.on() {
            return;
        }
        self.pause(now);
        self.buffer[seat] = Duration::from_secs(self.timer.buffer_secs.into());
        self.running = Some((seat, now));
    }

    /// The running clock stops, its time taken off: the buffer's first
    pub fn pause(&mut self, now: Instant) {
        if let Some((seat, since)) = self.running.take() {
            let (buffer, left) = self.at(seat, now.saturating_duration_since(since));
            self.buffer[seat] = buffer;
            self.left[seat] = left;
        }
    }

    /// The seat's buffer and time left after `spent` more
    fn at(&self, seat: Seat, spent: Duration) -> (Duration, Duration) {
        let from_buffer = spent.min(self.buffer[seat]);
        (self.buffer[seat] - from_buffer, self.left[seat].saturating_sub(spent - from_buffer))
    }

    fn now_of(&self, seat: Seat, now: Instant) -> (Duration, Duration) {
        match self.running {
            Some((s, since)) if s == seat => self.at(seat, now.saturating_duration_since(since)),
            _ => (self.buffer[seat], self.left[seat]),
        }
    }

    /// How long until the seat's time runs out, its buffer included; none without a timer
    pub fn remaining(&self, seat: Seat, now: Instant) -> Option<Duration> {
        if !self.on() {
            return None;
        }
        let (buffer, left) = self.now_of(seat, now);
        Some(buffer + left)
    }

    pub fn expired(&self, seat: Seat, now: Instant) -> bool {
        self.remaining(seat, now).is_some_and(|r| r.is_zero())
    }

    /// The view's `priorityTimeLeftSecs` and `bufferTimeLeft`: whole seconds, rounded up (Java's count reaches 0
    /// when the time runs out)
    pub fn view_secs(&self, seat: Seat, now: Instant) -> (i64, i64) {
        if !self.on() {
            return (NO_TIME_LIMIT, 0);
        }
        let (buffer, left) = self.now_of(seat, now);
        (secs_up(left), secs_up(buffer))
    }
}

impl Clock {
    /// The seat's time left in whole seconds, for the match's next game (Java's `priorityTimeLeft`); none without a
    /// timer
    pub fn left_secs(&self, seat: Seat, now: Instant) -> Option<u32> {
        self.on().then(|| self.view_secs(seat, now).0.clamp(0, u32::MAX as i64) as u32)
    }
}

fn secs_up(d: Duration) -> i64 {
    d.as_millis().div_ceil(1000) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    #[test]
    fn no_timer() {
        let mut c = Clock::new(Timer::default());
        let t = Instant::now();
        c.resume(0, t);
        assert!(!c.on());
        assert_eq!(c.remaining(0, t + s(9999)), None);
        assert!(!c.expired(0, t + s(9999)));
        assert_eq!(c.view_secs(0, t), (NO_TIME_LIMIT, 0));
    }

    #[test]
    fn counts_down_the_buffer_first_and_only_while_running() {
        let mut c = Clock::new(Timer { priority_secs: 60, buffer_secs: 5, ..Timer::default() });
        let t = Instant::now();
        assert_eq!(c.view_secs(0, t), (60, 0));
        c.resume(0, t);
        assert_eq!(c.view_secs(0, t), (60, 5));
        // 3 seconds in: from the buffer only
        assert_eq!(c.view_secs(0, t + s(3)), (60, 2));
        // 8 seconds in: the buffer is spent, then 3 seconds of the time
        assert_eq!(c.view_secs(0, t + s(8)), (57, 0));
        // the other seat's clock doesn't move
        assert_eq!(c.view_secs(1, t + s(8)), (60, 0));
        c.pause(t + s(8));
        // paused: nothing more goes
        assert_eq!(c.view_secs(0, t + s(100)), (57, 0));
        // started again: the buffer is full again
        c.resume(0, t + s(100));
        assert_eq!(c.view_secs(0, t + s(100)), (57, 5));
        assert_eq!(c.remaining(0, t + s(100)), Some(s(62)));
        // a part of a second shows as a whole one
        assert_eq!(c.view_secs(0, t + s(105) + Duration::from_millis(500)), (57, 0));
        assert_eq!(c.view_secs(0, t + s(106)), (56, 0));
    }

    #[test]
    fn a_match_keeps_the_time_left() {
        // the match's second game: seat 0 has 40 of the 60 seconds left from the first
        let c = Clock::new(Timer { priority_secs: 60, buffer_secs: 5, left: [Some(40), None] });
        let t = Instant::now();
        assert_eq!(c.view_secs(0, t), (40, 0));
        assert_eq!(c.view_secs(1, t), (60, 0));
        assert_eq!(c.left_secs(0, t), Some(40));
        assert_eq!(Clock::new(Timer::default()).left_secs(0, t), None);
    }

    #[test]
    fn runs_out() {
        let mut c = Clock::new(Timer { priority_secs: 10, buffer_secs: 2, ..Timer::default() });
        let t = Instant::now();
        c.resume(1, t);
        assert!(!c.expired(1, t + s(11)));
        assert!(c.expired(1, t + s(12)));
        assert_eq!(c.view_secs(1, t + s(30)), (0, 0));
        // resuming another seat stops this one where it was
        c.resume(0, t + s(5));
        assert_eq!(c.view_secs(1, t + s(30)), (7, 0));
        assert!(!c.expired(0, t + s(16)));
        assert!(c.expired(0, t + s(17)));
    }
}
