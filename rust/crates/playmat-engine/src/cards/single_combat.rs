//! Single Combat: "Each player chooses a creature or planeswalker they control, then sacrifices the rest. Players
//! can't cast creature or planeswalker spells until the end of your next turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SingleCombat.SingleCombatEffect", keep_one),
    Handler::effect("SingleCombat.SingleCombatRestrictionEffect", restrict),
];

fn creature_or_planeswalker(game: &Game, o: ObjId) -> bool {
    game.has_type(o, CardType::Creature) || game.has_type(o, CardType::Planeswalker)
}

/// From you on, each player `choose`s one of theirs (a not-target `TargetPermanent`, asked even with one; nothing
/// asked with none); then every other creature and planeswalker is sacrificed, in battlefield order
fn keep_one(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut kept = Vec::new();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let theirs: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| game.obj(*o).controller == p && creature_or_planeswalker(game, *o))
            .collect();
        if theirs.is_empty() {
            continue;
        }
        loop {
            match ask(game, d, p, &Query::Cards { objs: theirs.clone(), required: true }) {
                Answer::Obj(o) if theirs.contains(&o) => {
                    kept.push(o);
                    break;
                }
                Answer::Stop => return None,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        }
    }
    let rest: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| creature_or_planeswalker(game, *o) && !kept.contains(o)).collect();
    for o in rest {
        sacrifice(game, o);
    }
    Some(())
}

/// `CAST_SPELL` of a creature or planeswalker card by any player, `Duration.UntilEndOfYourNextTurn`: through your
/// next turn (two players, no extra turns)
fn restrict(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    for p in 0..game.seats() {
        game.player_mut(p).no_creatures_planeswalkers_until = Some(until);
    }
    Some(())
}
