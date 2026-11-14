//! Irencrag Feat: "Add seven {R}. You can cast only one more spell this turn."

use super::*;
use crate::card::{CardDef, EffectDef};

pub const HANDLERS: &[Handler] = &[
    Handler::effect("IrencragFeat.IrencragFeatEffect", apply)
        .effects(&["new BasicManaEffect(Mana.RedMana(7))", "new IrencragFeatCantCastEffect(0)"]),
    Handler::rule("IrencragFeat.IrencragFeatCantCastEffect", Rule::CantCast(cant_cast)).args(&["text"]),
];

/// `CastSpellLastTurnWatcher`: the spells `seat` cast this turn
fn cast(game: &Game, seat: Seat) -> usize {
    game.watch.spells.iter().filter(|s| s.caster == seat).count()
}

/// The mana, then the rule with the spells you've cast this turn so far as its `start`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let mut rule = cx.effects[1].clone();
    if let EffectDef::CardStatic { args, .. } = &mut rule {
        *args = vec![cast(game, cx.controller()).to_string()];
    }
    cx.run(game, d, &[rule]);
    Some(())
}

/// `CAST_SPELL` by you once you've cast a spell more than `start`
fn cant_cast(game: &Game, cx: &StaticCx, seat: Seat, _: ObjId, _: &CardDef) -> bool {
    seat == cx.controller && cast(game, seat) as i32 > cx.arg_int(0).unwrap_or(i32::MAX)
}
