//! A hosted game's replay, for the web client's History (`ReplayStore`): the file Java's `ReplayRecorder` writes for
//! the games it plays, `<gameId>.ndjson.gz.part`, one event a line: a board snapshot as a spectator sees it with the
//! human's hand beside it (`{"view", "hands": {name: hand}, "t"}`), or a game log line (`{"t", "log"}`). At the end
//! Java lists it (`RustHost.ended`, `ReplayStore.completeHosted`), as it does the replays of its own games.

use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::{json, Value};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

/// `ReplaySettings.MAX_EVENTS_PER_GAME`
const MAX_EVENTS: u32 = 20_000;

pub struct ReplayWriter {
    out: Option<GzEncoder<BufWriter<File>>>,
    started: Instant,
    last: Option<Value>,
    events: u32,
}

impl ReplayWriter {
    pub fn create(path: &Path) -> std::io::Result<ReplayWriter> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = BufWriter::new(File::create(path)?);
        Ok(ReplayWriter {
            out: Some(GzEncoder::new(file, Compression::default())),
            started: Instant::now(),
            last: None,
            events: 0,
        })
    }

    fn write(&mut self, mut event: Value) {
        if self.events >= MAX_EVENTS {
            return;
        }
        event["t"] = json!(self.started.elapsed().as_millis() as u64);
        let Some(out) = self.out.as_mut() else { return };
        if writeln!(out, "{event}").is_err() {
            // nothing more can be written; what was is kept
            self.out = None;
            return;
        }
        self.events += 1;
    }

    /// A board snapshot: the human's view, its hand moved beside it (a spectator's view has none); the same board
    /// again isn't written
    pub fn frame(&mut self, view: &Value, human: &str) {
        let mut view = view.clone();
        let hand = view.as_object_mut().and_then(|v| v.remove("myHand")).unwrap_or_else(|| json!({}));
        if let Some(v) = view.as_object_mut() {
            v.remove("events");
        }
        if self.last.as_ref() == Some(&view) {
            return;
        }
        self.last = Some(view.clone());
        let mut hands = serde_json::Map::new();
        hands.insert(human.to_string(), hand);
        self.write(json!({"view": view, "hands": hands}));
    }

    /// A game log line, the `ChatMessage` the client was sent
    pub fn log(&mut self, message: &Value) {
        self.write(json!({"log": message}));
    }

    /// The end: the file is complete, and the events in it (0: nothing, or it couldn't be written)
    pub fn finish(&mut self) -> u32 {
        let Some(out) = self.out.take() else { return 0 };
        match out.finish().and_then(|mut f| f.flush()) {
            Ok(()) => self.events,
            Err(_) => 0,
        }
    }
}
