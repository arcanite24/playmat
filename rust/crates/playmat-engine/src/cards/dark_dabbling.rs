//! Dark Dabbling: "Regenerate target creature. Draw a card. Spell mastery — If there are two or more instant and/or
//! sorcery cards in your graveyard, also regenerate each other creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DarkDabbling.DarkDabblingEffect", apply)];

/// With spell mastery, a regeneration shield on each creature you control but the target
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spells = game
        .player(you)
        .graveyard
        .iter()
        .filter(|c| game.obj(**c).is(CardType::Instant) || game.obj(**c).is(CardType::Sorcery))
        .count();
    if spells < 2 {
        return Some(());
    }
    let target = cx.obj(0);
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && Some(*o) != target)
        .collect();
    for o in creatures {
        game.obj_mut(o).regen_shields += 1;
    }
    Some(())
}
