//! `AddCounterChoiceSourceEffect(CounterType...)` (Flycatcher Giraffid, Helica Glider, Ferocious Tigorilla): "enters
//! with your choice of a vigilance counter or a reach counter on it".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AddCounterChoiceSourceEffect", apply)];

/// `ChoiceImpl` (required) of the counter kinds, named with their first letter upper case; one of the chosen kind
/// on the entering permanent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let kinds: Vec<String> = cx
        .args
        .iter()
        .filter_map(|a| a.trim().strip_prefix("CounterType."))
        .map(|k| k.to_lowercase().replace('_', " ").replace("p1p1", "+1/+1"))
        .collect();
    let o = cx.source_id();
    if kinds.len() < 2 || matches!(game.obj(o).zone, Zone::Graveyard | Zone::Exile | Zone::Hand) {
        return Some(());
    }
    let upper = |k: &str| {
        let mut c = k.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    };
    let choices: Vec<String> = kinds.iter().map(|k| upper(k)).collect();
    let q = Query::Choice { message: "Choose counter type".into(), choices: choices.clone() };
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
    let kind = kinds[choices.iter().position(|c| *c == chosen)?].clone();
    put_counters(game, o, &kind, 1, cx.controller());
    Some(())
}
