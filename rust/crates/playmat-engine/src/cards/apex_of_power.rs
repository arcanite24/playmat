//! Apex of Power: "Exile the top seven cards of your library. Until end of turn, you may cast spells from among
//! them. If this spell was cast from your hand, add ten mana of any one color."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ApexOfPower.ApexOfPowerSpellEffect", exile_seven),
    Handler::effect("ApexOfPower.ApexOfPowerManaEffect", ten_mana),
];

/// The top seven exiled; each nonland one castable by you until end of turn
fn exile_seven(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let turn = game.turn;
    for c in cards {
        if game.obj(c).zone == Zone::Exile && !game.has_type(c, CardType::Land) {
            let o = game.obj_mut(c);
            o.may_play = Some((you, turn));
            o.may_play_end_step = false;
            o.may_play_while = None;
            o.cast_only = true;
        }
    }
    Some(())
}

/// The spell cast from its controller's hand: `AddManaOfAnyColorEffect(10)`
fn ten_mana(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = game.obj(cx.source_id());
    if src.zone != Zone::Stack || src.cast_from != Some(Zone::Hand) {
        return Some(());
    }
    cx.run(game, d, &[EffectDef::AddManaOfAnyColorEffect { amount: 10 }]);
    (!d.stopped()).then_some(())
}
