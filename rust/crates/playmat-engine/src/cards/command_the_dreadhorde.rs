//! Command the Dreadhorde: "Choose any number of target creature and/or planeswalker cards in graveyards. Command
//! the Dreadhorde deals damage to you equal to the total mana value of those cards. Put them onto the battlefield
//! under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CommandTheDreadhorde.CommandTheDreadhordeEffect", apply)];

/// The damage (the legal targets' total mana value) to you, then `moveCards` of them all onto the battlefield under
/// your control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    let damage: i32 = cards.iter().map(|c| game.obj(*c).def.mana_value() as i32).sum();
    deal_damage(game, cx.source_id(), Target::Player(you), damage, false);
    let cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    if !cards.is_empty() && !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
