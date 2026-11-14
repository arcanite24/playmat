//! Wicked Guardian: "When Wicked Guardian enters, you may have it deal 2 damage to another creature you control. If
//! you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WickedGuardian.WickedGuardianEffect", apply)];

/// `player.choose(TargetPermanent(0, 1, another creature you control))`: none chosen does nothing
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != src && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let chosen = choose_cards_rounds(game, d, you, &possible, 0, 1)?;
    let Some(o) = chosen.first().copied() else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    deal_damage(game, src, Target::Obj(o), 2, false);
    draw(game, you, 1);
    Some(())
}
