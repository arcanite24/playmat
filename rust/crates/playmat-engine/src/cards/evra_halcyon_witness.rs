//! Evra, Halcyon Witness: "Lifelink. {4}: Exchange your life total with Evra, Halcyon Witness's power."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] = &[Handler::effect("EvraHalcyonWitness.EvraHalcyonWitnessEffect", apply)];

/// Nothing unless Evra is still a creature on the battlefield and the totals differ (nor when you can't gain the
/// life it would); your life becomes its power (`setLife`: a gain or a loss), then
/// `SetBasePowerSourceEffect(life, Duration.Custom)` for as long as it stays
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    if !cx.source_on_battlefield(game) || !game.has_type(me, CardType::Creature) {
        return Some(());
    }
    let amount = game.power_toughness(me).0;
    let life = game.player(you).life;
    if life == amount || (life < amount && game.player(you).cant_gain_life) {
        return Some(());
    }
    if amount > life {
        gain_life(game, you, amount - life);
    } else {
        lose_life(game, you, life - amount);
    }
    let at = game.timestamp();
    let b = crate::state::BasePtSet {
        id: me,
        zcc: cx.source.1,
        power: Some(Amount::Fixed(life)),
        toughness: None,
        source: me,
        controller: you,
        x: cx.item.x,
        at,
        until_eot: false,
    };
    game.base_pt_sets.push(b);
    Some(())
}
