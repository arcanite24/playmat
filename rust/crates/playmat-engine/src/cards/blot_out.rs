//! Blot Out: "Target opponent exiles a creature or planeswalker they control with the greatest mana value among
//! creatures and planeswalkers they control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlotOut.BlotOutEffect", apply)];

/// `choose(TargetPermanent(filter))`, not targeted and required: a lone choice is taken
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let theirs: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == p
                && (game.has_type(*o, CardType::Creature) || game.has_type(*o, CardType::Planeswalker))
        })
        .collect();
    let Some(most) = theirs.iter().map(|o| game.obj(*o).def.mana_value()).max() else { return Some(()) };
    let possible: Vec<ObjId> = theirs.iter().copied().filter(|o| game.obj(*o).def.mana_value() >= most).collect();
    let rule = item_rule(game, cx.item);
    if let Some(&o) = choose_cards(game, d, p, &possible, &possible, 1, 1, &rule)?.first() {
        game.move_to(o, Zone::Exile);
    }
    Some(())
}
