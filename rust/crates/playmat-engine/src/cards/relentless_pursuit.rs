//! Relentless Pursuit: "Reveal the top four cards of your library. You may put a creature card and/or a land card from
//! among them into your hand. Put the rest into your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RelentlessPursuit.RelentlessPursuitEffect", apply)];

/// The reveal, then `choose(TargetCardAndOrCardInLibrary(CREATURE, LAND))`: one `HumanPlayer.choose`, a pick at a time
/// among the cards that still leave a creature role and a land role apart (`hasSharedRoles`), until two are chosen or
/// none is left; the chosen cards to the hand, the rest to the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(4).copied().collect();
    if !cards.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: cards.iter().map(|c| (*c, Zone::Library)).collect(),
        });
    }
    let creature = |game: &Game, c: ObjId| game.obj(c).def.types.contains(&CardType::Creature);
    let land = |game: &Game, c: ObjId| game.obj(c).def.types.contains(&CardType::Land);
    let fits = |game: &Game, set: &[ObjId]| match set {
        [a] => creature(game, *a) || land(game, *a),
        [a, b] => (creature(game, *a) && land(game, *b)) || (land(game, *a) && creature(game, *b)),
        _ => set.is_empty(),
    };
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let possible: Vec<ObjId> = cards
            .iter()
            .copied()
            .filter(|c| {
                let mut set = chosen.clone();
                if !set.contains(c) {
                    set.push(*c);
                }
                fits(game, &set)
            })
            .collect();
        match ask(game, d, you, &Query::Cards { objs: possible.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if possible.contains(&o) => {
                chosen.push(o);
                let more = cards.iter().any(|c| {
                    !chosen.contains(c) && {
                        let mut set = chosen.clone();
                        set.push(*c);
                        fits(game, &set)
                    }
                });
                if chosen.len() >= 2 || !more {
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
    for c in &chosen {
        game.move_to(*c, Zone::Hand);
    }
    for c in cards {
        if !chosen.contains(&c) && game.obj(c).zone == Zone::Library {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}
