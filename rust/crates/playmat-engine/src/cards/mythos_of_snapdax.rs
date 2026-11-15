//! Mythos of Snapdax: "Each player chooses an artifact, a creature, an enchantment, and a planeswalker from among the
//! nonland permanents they control, then sacrifices the rest. If {B}{R} was spent to cast this spell, you choose the
//! permanents for each player instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MythosOfSnapdax.MythosOfSnapdaxEffect", apply)];

const TYPES: [CardType; 4] = [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Planeswalker];

/// Each player in turn order from you, for each type they control a nonland permanent of: that player (or you, with
/// {B}{R} spent) chooses one (`TargetPermanent`, not targeted); then every other nonland permanent is sacrificed, in
/// battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spent = game.obj(cx.source_id()).spent_colors;
    let you_choose = spent[2] > 0 && spent[3] > 0;
    let mut keep: Vec<ObjId> = Vec::new();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        for t in TYPES {
            let options: Vec<ObjId> = game
                .battlefield
                .iter()
                .copied()
                .filter(|o| game.obj(*o).controller == p && !game.has_type(*o, CardType::Land) && game.has_type(*o, t))
                .collect();
            if options.is_empty() {
                continue;
            }
            // `tryToAutoChoose` reads the spell's rule ("… then sacrifices the rest"): a lone permanent of the
            // chooser's own is still asked for, an opponent's is picked by itself
            let rule = item_rule(game, cx.item);
            let chooser = if you_choose { you } else { p };
            let chosen = choose_cards(game, d, chooser, &options, &options, 1, 1, &rule)?;
            keep.extend(chosen.first().copied());
        }
    }
    let doomed: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !game.has_type(*o, CardType::Land) && !keep.contains(o)).collect();
    for o in doomed {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}
