//! Temur Sabertooth: "{1}{G}: You may return another creature you control to its owner's hand. If you do, Temur
//! Sabertooth gains indestructible until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TemurSabertooth.TemurSabertoothEffect", apply)];

/// Only with another creature you control: `chooseUse`, then `chooseTarget(TargetPermanent(1, 1, …, notTarget))`;
/// it goes to its owner's hand and the Sabertooth gains indestructible
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let others: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != src && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if others.is_empty() || !choose_use(game, d, you, "Return another creature to hand?")? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &others, &others, 1, 1, &rule)?;
    if chosen.is_empty() {
        return Some(());
    }
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            game.move_to(o, Zone::Hand);
        }
    }
    if cx.source_on_battlefield(game) {
        grant(game, src, &AbilityDef::IndestructibleAbility);
    }
    Some(())
}
