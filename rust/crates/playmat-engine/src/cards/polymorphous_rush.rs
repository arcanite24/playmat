//! Polymorphous Rush: "Strive — This spell costs {1}{U} more to cast for each target beyond the first. Choose a
//! creature on the battlefield. Any number of target creatures you control each become a copy of that creature until
//! end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PolymorphousRush.PolymorphousRushCopyEffect", apply)];

/// The creature to copy (not targeted: "a creature on the battlefield (creature to copy)"), then each target still
/// on the battlefield, in target order, becomes a copy of it until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| game.has_type(*o, CardType::Creature)).collect();
    if creatures.is_empty() {
        return Some(());
    }
    let chosen =
        choose_cards(game, d, you, &creatures, &creatures, 1, 1, "a creature on the battlefield (creature to copy)")?;
    let Some(&from) = chosen.first() else { return Some(()) };
    if game.obj(from).zone != Zone::Battlefield {
        return Some(());
    }
    for to in cx.objs(0) {
        if game.obj(to).zone != Zone::Battlefield {
            continue;
        }
        let copy = game.obj(from).copiable();
        become_copy_until_end_of_turn(game, to, copy);
    }
    Some(())
}
