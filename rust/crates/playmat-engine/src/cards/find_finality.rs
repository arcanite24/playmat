//! Find // Finality: "Return up to two target creature cards from your graveyard to your hand. // You may put two
//! +1/+1 counters on a creature you control. Then all creatures get -4/-4 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FindFinality.FinalityEffect", apply)
    .effects(&["new BoostAllEffect(-4,-4,Duration.EndOfTurn)"])];

/// `player.choose` of `TargetControlledCreaturePermanent(0, 1)` (not targeted; not asked with no creature), two
/// +1/+1 counters on the one chosen, then -4/-4 to all creatures until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if !creatures.is_empty() {
        loop {
            match ask(game, d, you, &Query::Cards { objs: creatures.clone(), required: false }) {
                Answer::Obj(o) if creatures.contains(&o) => {
                    add_counters_by(game, o, "+1/+1", 2, Some(you));
                    break;
                }
                Answer::Stop => return None,
                Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
                _ => break,
            }
            if d.stopped() {
                return None;
            }
        }
    }
    cx.run(game, d, cx.effects);
    Some(())
}
