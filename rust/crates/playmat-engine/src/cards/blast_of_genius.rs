//! Blast of Genius: "Choose any target. Draw three cards, then discard a card. Blast of Genius deals damage equal to
//! the discarded card's mana value to that permanent or player."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlastOfGenius.BlastOfGeniusEffect", apply)];

/// `player.choose(Discard, TargetDiscard)`, that card discarded, then its mana value as damage
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 3);
    let hand = game.player(you).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &hand, &hand, 1, 1, "discard")?;
    let Some(c) = chosen.first().copied() else { return Some(()) };
    discard_card(game, you, c);
    game.discarded_cards();
    let damage = game.obj(c).def.mana_value() as i32;
    match cx.target(0) {
        Some(Target::Obj(o)) if game.obj(o).zone == Zone::Battlefield => {
            deal_damage(game, cx.source_id(), Target::Obj(o), damage, false);
        }
        Some(Target::Player(p)) if !game.player(p).lost => {
            deal_damage(game, cx.source_id(), Target::Player(p), damage, false);
        }
        _ => {}
    }
    Some(())
}
