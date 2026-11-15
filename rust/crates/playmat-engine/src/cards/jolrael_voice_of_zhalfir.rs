//! Jolrael, Voice of Zhalfir: "At the beginning of combat on your turn, up to one target land you control becomes an
//! X/X green and blue Bird creature with flying and haste until end of turn, where X is the number of cards in your
//! hand. It's still a land. Whenever a land creature you control deals combat damage to a player, draw a card."

use super::*;
use crate::card::{AbilityDef, Color, Duration, TokenDef};

pub const HANDLERS: &[Handler] = &[Handler::effect("JolraelVoiceOfZhalfir.JolraelVoiceOfZhalfirEffect", apply)];

/// X counted as it resolves; the land becomes an X/X green and blue Bird with flying and haste until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let count = game.player(cx.controller()).hand.len() as i32;
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let token = TokenDef {
        types: vec![CardType::Creature],
        colors: vec![Color::G, Color::U],
        subtypes: vec!["Bird".into()],
        power: count,
        toughness: count,
        abilities: vec![AbilityDef::FlyingAbility, AbilityDef::HasteAbility],
        ..Default::default()
    };
    let e = EffectDef::BecomesCreatureTargetEffect {
        token,
        duration: Duration::EndOfTurn,
        lose_abilities: false,
        still_land: true,
        named: None,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    Some(())
}
