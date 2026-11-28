//! Rampage of the Valkyries: "Whenever an Angel you control dies, each other player sacrifices a creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RampageOfTheValkyries.RampageOfTheValkyriesEffect", apply)];

/// Each other player in turn order with a creature chooses one (`TargetControlledCreaturePermanent`, not
/// targeted); then the chosen ones are sacrificed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut chosen = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let creatures: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
            .collect();
        if creatures.is_empty() {
            continue;
        }
        let rule = item_rule(game, cx.item);
        if let Some(o) = choose_cards(game, d, p, &creatures, &creatures, 1, 1, &rule)?.first() {
            chosen.push(*o);
        }
    }
    for o in chosen {
        sacrifice(game, o);
    }
    Some(())
}
