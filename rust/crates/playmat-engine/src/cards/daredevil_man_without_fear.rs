//! Daredevil, Man Without Fear: "Vigilance, haste. Radar Sense — You may look at the top card of your library any
//! time. Whenever you attack, you may exile the top card of your library. If that card is a Hero card, Daredevil
//! gets +2/+1 until end of turn. You may play that card this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DaredevilManWithoutFear.DaredevilManWithoutFearEffect", apply)];

/// The top card exiled; still in exile: a Hero card gives the source +2/+1 until end of turn, and you may play it
/// this turn (`PlayFromNotOwnHandZoneTargetEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    if game.has_subtype(card, "Hero") {
        let e = EffectDef::BoostSourceEffect {
            power: crate::card::Amount::Fixed(2),
            toughness: crate::card::Amount::Fixed(1),
            duration: crate::card::Duration::EndOfTurn,
        };
        cx.run(game, d, &[e]);
        if d.stopped() {
            return None;
        }
    }
    let turn = game.turn;
    game.obj_mut(card).may_play = Some((you, turn));
    Some(())
}
