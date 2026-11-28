//! Seasons Past: "Return any number of cards with different mana values from your graveyard to your hand. Put
//! Seasons Past on the bottom of its owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SeasonsPast.SeasonsPastEffect", apply)];

/// `controller.choose(outcome, SeasonsPastTarget, …)`: one `HumanPlayer.choose`, a pick at a time among the cards
/// whose mana value no chosen card has, until the player is done or none is left (`isChoiceCompleted`); then the
/// chosen cards go to the hand together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mv = |game: &Game, c: ObjId| game.obj(c).def.mana_value();
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let used: Vec<_> = chosen.iter().map(|c| mv(game, *c)).collect();
        let possible: Vec<ObjId> =
            game.player(you).graveyard.iter().copied().filter(|c| !used.contains(&mv(game, *c))).collect();
        match ask(game, d, you, &Query::Cards { objs: possible.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if possible.contains(&o) => {
                chosen.push(o);
                let m = mv(game, o);
                if possible.iter().all(|c| *c == o || mv(game, *c) == m) {
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
    for c in chosen {
        if game.obj(c).zone == Zone::Graveyard {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}
