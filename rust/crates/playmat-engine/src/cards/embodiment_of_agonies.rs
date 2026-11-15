//! Embodiment of Agonies: "Embodiment of Agonies enters with a +1/+1 counter on it for each different mana cost
//! among nonland cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("EmbodimentOfAgonies.EmbodimentOfAgoniesValue", costs)];

/// each nonland card's mana cost symbols, its colored ones sorted and its generic summed last; empty ones (no
/// mana cost) don't count
fn costs(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut seen: Vec<String> = Vec::new();
    for c in &game.player(you).graveyard {
        if game.has_type(*c, CardType::Land) {
            continue;
        }
        let text = &game.obj(*c).def.cost.text;
        let mut symbols: Vec<String> = Vec::new();
        let (mut generic, mut has_generic) = (0u32, false);
        for s in text.split('}').filter(|s| !s.is_empty()) {
            let s = format!("{s}}}");
            let inner = &s[1..s.len() - 1];
            if !inner.is_empty() && inner.chars().all(|ch| ch.is_ascii_digit()) {
                generic += inner.parse::<u32>().unwrap_or(0);
                has_generic = true;
            } else {
                symbols.push(s);
            }
        }
        symbols.sort();
        if has_generic {
            symbols.push(format!("{{{generic}}}"));
        }
        let key = symbols.concat();
        if !key.is_empty() && !seen.contains(&key) {
            seen.push(key);
        }
    }
    seen.len() as i32
}
