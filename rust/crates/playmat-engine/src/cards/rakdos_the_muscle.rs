//! Rakdos, the Muscle: "Whenever you sacrifice another creature, exile cards equal to its mana value from the top of
//! target player's library. Until your next end step, you may play those cards, and mana of any type can be spent to
//! cast those spells."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RakdosTheMuscle.RakdosTheMuscleEffect", exile_and_play)];

/// The sacrificed creature's mana value (as it last was) of cards off the top of the target player's library into
/// exile; those still there are yours to play until your next end step (`Duration.UntilYourNextEndStep`), with mana
/// of any type
fn exile_and_play(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (Some(player), Some(sacrificed)) = (cx.player(0), cx.item.sacrificed_permanent) else { return Some(()) };
    let amount = game.obj(sacrificed).def.mana_value() as usize;
    if amount == 0 {
        return Some(());
    }
    let cards: Vec<ObjId> = game.player(player).library.iter().take(amount).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    // this turn's end step if it's yours and hasn't begun, else your next turn's
    let until = match game.active {
        Some(a) if a == you && !matches!(game.step, Some(Step::EndTurn | Step::Cleanup)) => game.turn,
        Some(a) if a == you => game.turn + game.seats() as u32,
        _ => game.turn + 1,
    };
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            let o = game.obj_mut(c);
            o.may_play = Some((you, until));
            o.may_play_end_step = true;
            o.may_play_while = None;
            o.any_mana = true;
        }
    }
    Some(())
}
