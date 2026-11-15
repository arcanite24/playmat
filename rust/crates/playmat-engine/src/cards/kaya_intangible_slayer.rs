//! Kaya, Intangible Slayer: "Hexproof. +2: Each opponent loses 3 life and you gain 3 life. 0: You draw two cards.
//! Then each opponent may scry 1. −3: Exile target creature or enchantment. If it wasn't an Aura, create a token
//! that's a copy of it, except it's a 1/1 white Spirit creature with flying in addition to its other types."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("KayaIntangibleSlayer.KayaIntangibleSlayerScryEffect", scry_each),
    Handler::effect("KayaIntangibleSlayer.KayaIntangibleSlayerExileEffect", exile_and_copy),
];

/// Each opponent in turn order: "Scry 1?", then the scry
fn scry_each(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost || !Game::whose("opponent", you, p) {
            continue;
        }
        if choose_use(game, d, p, "Scry 1?")? {
            cx.run_as(game, d, &[EffectDef::ScryEffect { amount: Amount::Fixed(1) }], p);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}

/// The target exiled; unless it was an Aura, a token copy of it as it last was on the battlefield (the saved
/// permanent), as `applyAdditionsToToken` orders the exceptions: a creature, flying, 1/1, a Spirit, only white
fn exile_and_copy(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let aura = game.has_subtype(o, "Aura");
    let saved = game.obj(o).copiable();
    game.move_to(o, Zone::Exile);
    if aura {
        return Some(());
    }
    token_copy_modified(game, d, o, you, 1, &|def: &mut CardDef| {
        *def = (*saved).clone();
        if !def.types.contains(&CardType::Creature) {
            def.types.push(CardType::Creature);
        }
        def.abilities.push(AbilityDef::FlyingAbility);
        def.remove_pt_cda();
        def.power = 1;
        def.toughness = 1;
        if !def.subtypes.iter().any(|s| s == "Spirit") {
            def.subtypes.push("Spirit".into());
        }
        def.colors = vec![Color::W];
    });
    (!d.stopped()).then_some(())
}
