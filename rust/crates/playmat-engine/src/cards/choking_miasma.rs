//! Choking Miasma: "Kicker {G}. If this spell was kicked, put a +1/+1 counter on a creature you control. All
//! creatures get -2/-2 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChokingMiasma.ChokingMiasmaEffect", apply)];

/// Kicked, with a creature you control: you choose one (not targeted) for the counter
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.item.kicked {
        return Some(());
    }
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&o) = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first() else { return Some(()) };
    add_counters_by(game, o, "+1/+1", 1, Some(you));
    Some(())
}
