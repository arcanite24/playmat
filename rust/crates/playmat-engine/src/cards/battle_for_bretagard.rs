//! Battle for Bretagard: "I — Create a 1/1 white Human Warrior creature token. II — Create a 1/1 green Elf Warrior
//! creature token. III — Choose any number of artifact tokens and/or creature tokens you control with different
//! names. For each of them, create a token that's a copy of it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BattleForBretagard.BattleForBretagardEffect", apply)];

/// `choose(BattleForBretagardTarget)`: any number of artifact or creature tokens you control, each pick leaving out
/// the ones sharing a name with one chosen (`possibleTargets`); done once nothing more can be chosen
/// (`isChoiceCompleted`) or the player says so. Then a token copy of each, in the order chosen
/// (`CreateTokenCopyTargetEffect` with a fixed target).
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let fits = |game: &Game, o: ObjId| {
        let ob = game.obj(o);
        ob.controller == you
            && ob.token
            && (game.has_type(o, CardType::Artifact) || game.has_type(o, CardType::Creature))
    };
    let possible = |game: &Game, chosen: &[ObjId]| -> Vec<ObjId> {
        let names: Vec<String> =
            chosen.iter().map(|c| game.obj(*c).name().to_string()).filter(|n| !n.is_empty()).collect();
        game.battlefield
            .iter()
            .copied()
            .filter(|o| fits(game, *o) && !names.iter().any(|n| n == game.obj(*o).name()))
            .collect()
    };
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let can = possible(game, &chosen);
        match ask(game, d, you, &Query::Cards { objs: can.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if can.contains(&o) => {
                chosen.push(o);
                if possible(game, &chosen).is_empty() {
                    break;
                }
            }
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ => break,
        }
        if d.stopped() {
            return None;
        }
    }
    for t in chosen {
        token_copy(game, d, t, you, None);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
