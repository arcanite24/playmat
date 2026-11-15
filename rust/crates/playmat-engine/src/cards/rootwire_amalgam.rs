//! Rootwire Amalgam: "Prototype {1}{G} — 2/3. {3}{G}{G}, Sacrifice this creature: Create an X/X colorless Golem
//! artifact creature token, where X is three times this creature's power. It gains haste until end of turn. Activate
//! only as a sorcery."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] = &[Handler::effect("RootwireAmalgam.RootwireAmalgamEffect", apply)
    .tokens(&["GolemXXToken"])
    .effects(&["new GainAbilityTargetEffect(HasteAbility.getInstance())"])];

/// `getSourcePermanentOrLKI`'s power three times over, the token, then haste until end of turn for it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let power = if cx.source_on_battlefield(game) {
        game.power_toughness(cx.source_id()).0
    } else {
        cx.item.lki_power.unwrap_or_else(|| game.power_toughness(cx.source_id()).0)
    };
    let first = game.objs.len();
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: Amount::Fixed(3 * power) };
    cx.run_on(game, d, &[e], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<Target> = (first..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .map(Target::Obj)
        .collect();
    if !made.is_empty() {
        cx.run_on(game, d, cx.effects, made);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
