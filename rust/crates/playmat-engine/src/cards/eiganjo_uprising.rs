//! Eiganjo Uprising: "Create X 2/2 white Samurai creature tokens with vigilance. They gain menace and haste until end
//! of turn. Each opponent creates X minus one 2/2 white Samurai creature tokens with vigilance."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EiganjoUprising.EiganjoUprisingEffect", apply)
    .effects(&["new CreateTokenEffect(new SamuraiToken())"])];

fn tokens(cx: &EffectCx, n: i32) -> EffectDef {
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
        *amount = crate::card::Amount::Fixed(n);
    }
    e
}

/// X as cast (nothing below 1): your tokens, menace and then haste on them until end of turn, then each opponent's
/// X - 1 in turn order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    if x < 1 {
        return Some(());
    }
    let you = cx.controller();
    let before = game.objs.len();
    cx.run_on(game, d, &[tokens(cx, x)], Vec::new());
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = (before..game.objs.len())
        .map(|i| ObjId(i as u32))
        .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
        .collect();
    for a in [AbilityDef::MenaceAbility, AbilityDef::HasteAbility] {
        for o in &made {
            grant(game, *o, &a);
        }
    }
    if x < 2 {
        return Some(());
    }
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        cx.run_as(game, d, &[tokens(cx, x - 1)], p);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
