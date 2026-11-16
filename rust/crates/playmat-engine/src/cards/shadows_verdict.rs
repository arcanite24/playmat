//! Shadows' Verdict: "Exile all creatures and planeswalkers with mana value 3 or less from the battlefield and all
//! creature and planeswalker cards with mana value 3 or less from all graveyards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShadowsVerdict.ShadowsVerdictEffect", apply)];

/// the permanents in battlefield order, then each graveyard's cards (players in range from you), all exiled together
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let small = |game: &Game, o: ObjId| {
        (game.has_type(o, CardType::Creature) || game.has_type(o, CardType::Planeswalker))
            && (game.obj(o).def.mana_value() as i32) < 4
    };
    let mut cards: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| small(game, *o)).collect();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        cards.extend(game.player(p).graveyard.iter().copied().filter(|c| small(game, *c)));
    }
    for c in cards {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
