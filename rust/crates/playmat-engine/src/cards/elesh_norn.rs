//! Elesh Norn: "Whenever a source an opponent controls deals damage to you or a permanent you control, that source's
//! controller loses 2 life unless they pay {1}."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("EleshNorn.DoUnlessTargetPlayerOrTargetsControllerPaysEffect", apply)
        .effects(&["new LoseLifeTargetEffect(2)"]),
    Handler::effect("EleshNorn.TheArgentEtchingsEffect", argent_etchings).effects(&["new IncubateEffect(2)"]),
];

/// The Argent Etchings' chapter I: "incubate 2 five times, then transform all Incubator tokens you control" (each
/// Incubator token you control, in battlefield order, `getActivePermanents`)
fn argent_etchings(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for _ in 0..5 {
        cx.run(game, d, &cx.effects[..1]);
        if d.stopped() {
            return None;
        }
    }
    let you = cx.controller();
    let incubators: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.obj(*o).token && game.has_subtype(*o, "Incubator"))
        .collect();
    for o in incubators {
        if game.obj(o).zone == Zone::Battlefield {
            transform(game, o);
        }
    }
    Some(())
}

/// `DoUnlessTargetPlayerOrTargetsControllerPaysEffect(lose 2 life, {1})`: the source's controller (the pointer) is
/// asked (a mana cost can always be paid) and pays; the life loss unless they paid
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(player) = cx.player(0) else { return Some(()) };
    let mut paid = false;
    if choose_use(game, d, player, "Pay {1} to prevent (that source's controller loses 2 life)?")? {
        paid = pay_mana(game, d, player, &ManaCost::parse("{1}").ok()?);
        if d.stopped() {
            return None;
        }
    }
    if !paid {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Player(player)]);
    }
    Some(())
}
