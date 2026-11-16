//! Inventive Iteration: "I — Return up to one target creature or planeswalker to its owner's hand. II — Return an
//! artifact card from your graveyard to your hand. If you can't, draw a card. III — Exile this Saga, then return it
//! to the battlefield transformed under your control." Living Breakthrough: "Flying. Whenever you cast a spell,
//! your opponents can't cast spells with the same mana value as that spell until your next turn."

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("InventiveIteration.InventiveIterationEffect", apply),
    Handler::rule("InventiveIteration.LivingBreakthroughEffect", Rule::CantCastOnStack(same_mana_value))
        .until_your_next_turn()
        .on_start(spell_mana_value),
];

/// With no artifact card in your graveyard, you draw a card; with one, it returns (`randomFromCollection` of one
/// asks the RNG nothing); with more, you choose one (`TargetCardInYourGraveyard`, not targeted, `Player.choose`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let artifacts: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|c| game.obj(*c).def.is(CardType::Artifact)).collect();
    let card = match artifacts.len() {
        0 => {
            draw(game, you, 1);
            return Some(());
        }
        1 => Some(artifacts[0]),
        _ => choose_cards_rounds(game, d, you, &artifacts, 1, 1)?.first().copied(),
    };
    if let Some(c) = card {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}

/// `init`: the mana value of the spell that was cast (`getValue("spellCast")`, X included), kept as the effect's
/// amount (-1 without one)
fn spell_mana_value(game: &mut Game, _d: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let spell = game.resolving_item.as_ref().and_then(|i| i.cast_spell);
    let mv = spell.and_then(|s| game.watch.spells.iter().rev().find(|w| w.card == s)).map_or(-1, |w| w.mv);
    cx.set_left(game, mv);
    Some(cx.targets.to_vec())
}

/// `CAST_SPELL_LATE`: an opponent's spell on the stack with that mana value
fn same_mana_value(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, face: &CardDef) -> bool {
    if !cx.opponent(seat) {
        return false;
    }
    let x = game.stack.iter().rev().find(|t| t.card == id && t.ability.is_none()).map_or(0, |t| t.x);
    let mv = face.mana_value() as i32 + x * face.printed().x as i32;
    mv == cx.left(game, -1)
}
