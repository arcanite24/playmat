//! Grimdancer: "Grimdancer enters the battlefield with your choice of two different counters on it from among
//! menace, deathtouch, and lifelink."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Grimdancer.GrimdancerEffect", apply)];

const CHOICES: [(&str, &str, &str); 3] = [
    ("Menace and deathtouch", "menace", "deathtouch"),
    ("Menace and lifelink", "menace", "lifelink"),
    ("Deathtouch and lifelink", "deathtouch", "lifelink"),
];

/// `ChoiceImpl` (required) "Choose two abilities", then the two counters on the entering permanent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.source_id();
    if matches!(game.obj(o).zone, Zone::Graveyard | Zone::Exile | Zone::Hand) {
        return Some(());
    }
    let choices: Vec<String> = CHOICES.iter().map(|c| c.0.to_string()).collect();
    let q = Query::Choice { message: "Choose two abilities".into(), choices: choices.clone() };
    let chosen = loop {
        match ask(game, d, cx.controller(), &q) {
            Answer::Text(t) if choices.contains(&t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let (_, a, b) = CHOICES.iter().find(|c| c.0 == chosen)?;
    put_counters(game, o, a, 1, cx.controller());
    put_counters(game, o, b, 1, cx.controller());
    Some(())
}
