//! Every answer a game was played with, so the game can be replayed without its searches (Mana@Home's replay check,
//! docs/swarm/Design.md, "Trust and verification").
//!
//! A game is fixed by its deal (decks, seed, who plays first) and the answers both seats gave: the engine's own
//! randomness comes from the game's seed, and the searches only ever play on copies. So a log of the answers, in
//! order, replays the game exactly, and costs no search. `Logged` records the log around any decider, `Replayer`
//! plays a log back, and `encode` / `decode` turn it into one line of text (about 4 bytes an answer).

use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::state::{Game, ObjId, Seat, Step};

/// One thing a seat decided: a mulligan (`Keep`), a card put on the bottom, or an answer to the engine.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    /// a mulligan decision: whether the seat took another hand
    Mulligan(Seat, bool),
    /// the card a seat put on the bottom after a mulligan
    Bottom(Seat, ObjId),
    Answer(Seat, Answer),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnswerLog {
    pub entries: Vec<Entry>,
}

impl AnswerLog {
    /// The log as one line: entries joined by spaces, each the seat's digit and a short code
    pub fn encode(&self) -> String {
        let mut out = String::with_capacity(self.entries.len() * 4);
        for (i, e) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            match e {
                Entry::Mulligan(s, m) => out.push_str(&format!("{s}m{}", *m as u8)),
                Entry::Bottom(s, id) => out.push_str(&format!("{s}b{}", id.0)),
                Entry::Answer(s, a) => {
                    out.push_str(&s.to_string());
                    out.push_str(&encode_answer(a));
                }
            }
        }
        out
    }

    pub fn decode(text: &str) -> Result<AnswerLog, String> {
        let mut entries = Vec::new();
        for tok in text.split(' ').filter(|t| !t.is_empty()) {
            let digits = tok.chars().take_while(|c| c.is_ascii_digit()).count();
            let seat: Seat = tok[..digits].parse().map_err(|_| format!("no seat in {tok:?}"))?;
            let rest = &tok[digits..];
            let e = match rest.as_bytes().first() {
                Some(b'm') => Entry::Mulligan(seat, &rest[1..] == "1"),
                Some(b'b') => Entry::Bottom(seat, ObjId(num(&rest[1..], tok)?)),
                _ => Entry::Answer(seat, decode_answer(rest).ok_or_else(|| format!("bad answer {tok:?}"))?),
            };
            entries.push(e);
        }
        Ok(AnswerLog { entries })
    }

    /// sha256 of the encoded log, in hex: two games with the same hash were played the same way
    pub fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(self.encode().as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
    }
}

fn num<T: std::str::FromStr>(s: &str, tok: &str) -> Result<T, String> {
    s.parse().map_err(|_| format!("bad number in {tok:?}"))
}

/// Text and mana types can hold spaces: `%` escapes them
fn escape(s: &str) -> String {
    s.replace('%', "%25").replace(' ', "%20")
}

fn unescape(s: &str) -> String {
    s.replace("%20", " ").replace("%25", "%")
}

fn encode_answer(a: &Answer) -> String {
    match a {
        Answer::Bool(b) => format!("B{}", *b as u8),
        Answer::Int(n) => format!("I{n}"),
        Answer::Text(t) => format!("X{}", escape(t)),
        Answer::ManaType(t) => format!("M{}", escape(t)),
        Answer::Player(p) => format!("P{p}"),
        Answer::Obj(id) => format!("O{}", id.0),
        Answer::Ability(id, k) => format!("A{}.{k}", id.0),
        Answer::Trigger(k) => format!("T{k}"),
        Answer::ModeDone => "D".into(),
        Answer::ModeCancel => "C".into(),
        Answer::Mode(k) => format!("S{k}"),
        Answer::Unknown => "U".into(),
        Answer::Stop => "Z".into(),
    }
}

fn decode_answer(s: &str) -> Option<Answer> {
    let (head, rest) = s.split_at(s.char_indices().nth(1).map_or(s.len(), |(i, _)| i));
    Some(match head {
        "B" => Answer::Bool(rest == "1"),
        "I" => Answer::Int(rest.parse().ok()?),
        "X" => Answer::Text(unescape(rest)),
        "M" => Answer::ManaType(unescape(rest)),
        "P" => Answer::Player(rest.parse().ok()?),
        "O" => Answer::Obj(ObjId(rest.parse().ok()?)),
        "A" => {
            let (id, k) = rest.split_once('.')?;
            Answer::Ability(ObjId(id.parse().ok()?), k.parse().ok()?)
        }
        "T" => Answer::Trigger(rest.parse().ok()?),
        "D" => Answer::ModeDone,
        "C" => Answer::ModeCancel,
        "S" => Answer::Mode(rest.parse().ok()?),
        "U" => Answer::Unknown,
        "Z" => Answer::Stop,
        _ => return None,
    })
}

/// Records every answer `inner` gives into `log`
pub struct Logged<'a> {
    pub inner: &'a mut dyn Decider,
    pub log: &'a mut AnswerLog,
}

impl Decider for Logged<'_> {
    fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
        let a = self.inner.answer(game, seat, query);
        self.log.entries.push(Entry::Answer(seat, a.clone()));
        a
    }

    fn traced(&mut self, game: &Game) {
        self.inner.traced(game)
    }

    fn stopped(&self) -> bool {
        self.inner.stopped()
    }

    fn stop_after(&mut self, game: &Game, step: Step) -> bool {
        self.inner.stop_after(game, step)
    }
}

/// Plays a log back. Anything that doesn't fit (the wrong seat asked, an answer no client could give to the question,
/// the log running out) stops the game and is kept in `problem`.
pub struct Replayer<'a> {
    pub log: &'a AnswerLog,
    pub pos: usize,
    pub problem: Option<String>,
}

impl<'a> Replayer<'a> {
    pub fn new(log: &'a AnswerLog) -> Replayer<'a> {
        Replayer { log, pos: 0, problem: None }
    }

    fn next(&mut self) -> Option<&'a Entry> {
        let e = self.log.entries.get(self.pos);
        self.pos += 1;
        e
    }

    fn fail(&mut self, why: String) {
        if self.problem.is_none() {
            self.problem = Some(why);
        }
    }

    /// The logged mulligan decision for `seat`
    pub fn mulligan(&mut self, seat: Seat) -> bool {
        match self.next() {
            Some(Entry::Mulligan(s, m)) if *s == seat => *m,
            other => {
                let why = format!("entry {}: wanted seat {seat}'s mulligan, log has {other:?}", self.pos - 1);
                self.fail(why);
                false
            }
        }
    }

    /// The logged bottom card for `seat`
    pub fn bottom(&mut self, game: &Game, seat: Seat) -> ObjId {
        match self.next() {
            Some(Entry::Bottom(s, id)) if *s == seat => *id,
            other => {
                let why = format!("entry {}: wanted seat {seat}'s bottom card, log has {other:?}", self.pos - 1);
                self.fail(why);
                crate::mulligan::rule_bottom(game, seat)
            }
        }
    }

    /// Every entry was used
    pub fn finished(&self) -> bool {
        self.pos == self.log.entries.len()
    }
}

impl Decider for Replayer<'_> {
    fn answer(&mut self, _game: &Game, seat: Seat, query: &Query) -> Answer {
        if self.problem.is_some() {
            return Answer::Stop;
        }
        match self.next() {
            Some(Entry::Answer(s, a)) if *s == seat && query.accepts(a) => a.clone(),
            other => {
                let why = format!("entry {}: seat {seat} asked {query:?}, log has {other:?}", self.pos - 1);
                self.fail(why);
                Answer::Stop
            }
        }
    }

    fn stopped(&self) -> bool {
        self.problem.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_survives_encoding() {
        let log = AnswerLog {
            entries: vec![
                Entry::Mulligan(0, true),
                Entry::Bottom(0, ObjId(41)),
                Entry::Mulligan(1, false),
                Entry::Answer(0, Answer::Bool(false)),
                Entry::Answer(1, Answer::Obj(ObjId(7))),
                Entry::Answer(1, Answer::Ability(ObjId(7), 2)),
                Entry::Answer(0, Answer::Text("a 100% text".into())),
                Entry::Answer(0, Answer::ManaType("Green".into())),
                Entry::Answer(1, Answer::Int(-3)),
                Entry::Answer(1, Answer::Player(0)),
                Entry::Answer(0, Answer::Trigger(1)),
                Entry::Answer(0, Answer::Mode(2)),
                Entry::Answer(0, Answer::ModeDone),
                Entry::Answer(0, Answer::ModeCancel),
                Entry::Answer(0, Answer::Unknown),
                Entry::Answer(0, Answer::Stop),
            ],
        };
        let text = log.encode();
        assert_eq!(AnswerLog::decode(&text).unwrap(), log);
        assert_eq!(log.hash().len(), 64);
    }
}
