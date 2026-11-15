//! Summoner's Grimoire: equipped creature has "Whenever this creature attacks, you may put a creature card from your
//! hand onto the battlefield. If that card is an enchantment card, it enters tapped and attacking."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SummonersGrimoire.SummonersGrimoireEffect", apply)];

/// `TargetCardInHand(0, 1, creature card)` chosen from the hand (no yes/no first, and asked even with no creature
/// card in it); an enchantment card enters tapped and then joins combat (`addAttackingCreature`, its defender asked
/// when there's a choice)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let possible: Vec<ObjId> = hand.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)).collect();
    let Some(&c) = choose_cards(game, d, you, &hand, &possible, 0, 1, "a creature card")?.first() else {
        return Some(());
    };
    let enchantment = game.obj(c).is(CardType::Enchantment);
    let how = if enchantment { crate::card::PutCards::BattlefieldTapped } else { crate::card::PutCards::Battlefield };
    if !put_cards(game, d, you, &[c], how) {
        return None;
    }
    if enchantment && game.obj(c).zone == Zone::Battlefield && game.obj(c).is(CardType::Creature) {
        let defender = game.opponent(you);
        let pw = choose_defender(game, d, you, defender)?;
        let o = game.obj_mut(c);
        o.attacking = Some(defender);
        o.attacking_pw = pw;
        join_combat(game, c);
    }
    Some(())
}
