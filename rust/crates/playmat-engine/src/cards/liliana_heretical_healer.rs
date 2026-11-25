//! Liliana, Heretical Healer: "Lifelink. Whenever another nontoken creature you control dies, exile Liliana, Heretical
//! Healer, then return her to the battlefield transformed under her owner's control. If you do, create a 2/2 black
//! Zombie creature token." Her back face's emblem: "Whenever a creature dies, return it to the battlefield under your
//! control at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LilianaHereticalHealer.ExileAndReturnSourceEffect", flip).tokens(&["ZombieToken"]),
    Handler::effect("LilianaDefiantNecromancerEmblemEffect", emblem_return),
];

/// `ExileAndReturnSourceEffect(BATTLEFIELD_TRANSFORMED, SHE, false, CreateTokenEffect(ZombieToken))`: exiled and
/// returned transformed under her owner's control; the token only when her card is on the battlefield after that
fn flip(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::ExileAndReturnSourceEffect { your_control: false, tapped: false, transformed: true };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    let me = cx.source_id();
    if game.obj(me).zone == Zone::Battlefield && !game.obj(me).token {
        if let Some(t) = cx.tokens.first() {
            cx.create_tokens(game, d, t, 1);
        }
    }
    Some(())
}

/// the creature card that died: a delayed "at the beginning of the next end step" return of that card from the
/// graveyard under the emblem's controller (`ReturnFromGraveyardToBattlefieldTargetEffect`, `FixedTarget(card)`)
fn emblem_return(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((card, _)) = cx.item.died else { return Some(()) };
    let e = EffectDef::CreateNextEndStepDelayedEffect {
        effects: vec![EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect {
            tapped: false,
            owner: false,
            counters: None,
            attacking: false,
        }],
        rule: "At the beginning of the next end step, return that card to the battlefield at the beginning of the \
               next end step."
            .into(),
        fixed_targets: true,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(card)]);
    Some(())
}
