//! Break Under Pressure: "Target opponent sacrifices a creature or planeswalker with the greatest mana value among
//! creatures and planeswalkers they control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BreakUnderPressure.BreakUnderPressureEffect", apply)];

/// The opponent chooses one of theirs (`TargetPermanent`, not targeted) whose mana value is the greatest and
/// sacrifices it
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
    let mv = |o: &ObjId| game.obj(*o).def.mana_value();
    let Some(most) = theirs.iter().map(mv).max() else { return Some(()) };
    let greatest: Vec<ObjId> = theirs.iter().copied().filter(|o| mv(o) >= most).collect();
    let chosen = choose_cards(game, d, p, &greatest, &greatest, 1, 1, &item_rule(game, cx.item))?;
    if let Some(o) = chosen.first().copied() {
        sacrifice(game, o);
    }
    Some(())
}
