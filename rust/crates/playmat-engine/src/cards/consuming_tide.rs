//! Consuming Tide: "Each player chooses a nonland permanent they control. Return all nonland permanents not chosen
//! this way to their owners' hands. Then you draw a card for each opponent who has more cards in their hand than you."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ConsumingTide.ConsumingTideEffect", apply)];

/// Each player in turn order from you chooses (Java's `TargetNonlandPermanent`, not targeted: any nonland
/// permanent, not only their own); the rest return in battlefield order; then the draw
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let nonland = |game: &Game| -> Vec<ObjId> {
        game.battlefield.iter().copied().filter(|o| !game.has_type(*o, CardType::Land)).collect()
    };
    let mut chosen = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let possible = nonland(game);
        if possible.is_empty() {
            continue;
        }
        if let Some(&o) = choose_cards(game, d, p, &possible, &possible, 1, 1, &rule)?.first() {
            chosen.push(o);
        }
    }
    for o in nonland(game) {
        if !chosen.contains(&o) {
            game.move_to(o, Zone::Hand);
        }
    }
    let mine = game.player(you).hand.len();
    let n = (1..game.seats())
        .map(|i| (you + i) % game.seats())
        .filter(|p| !game.player(*p).lost && game.player(*p).hand.len() > mine)
        .count();
    if n > 0 {
        draw(game, you, n);
    }
    Some(())
}
