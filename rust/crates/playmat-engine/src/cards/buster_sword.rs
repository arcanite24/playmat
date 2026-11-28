//! Buster Sword: "Whenever equipped creature deals combat damage to a player, draw a card, then you may cast a spell
//! from your hand with mana value less than or equal to that damage without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BusterSword.BusterSwordEffect", apply)];

/// The parts of a hand card that may be cast for free with mana value `n` or less (`getCastableComponents`: a spell,
/// not a land, with its targets possible): the card itself and its adventure
fn parts(game: &Game, seat: Seat, card: ObjId, n: i32) -> (bool, bool) {
    let def = game.obj(card).def.clone();
    let fits = |face: &CardDef| {
        !face.is(CardType::Land) && face.mana_value() as i32 <= n && {
            let free = CardDef { cost: ManaCost::default(), ..face.clone() };
            has_targets(game, seat, card, &free)
        }
    };
    (fits(&def), def.adventure.as_ref().is_some_and(|a| fits(a)))
}

/// `CardUtil.castSpellWithAttributesForFree` over the hand: one card among those with a part to cast (chosen when
/// there are several, "up to one"), "Cast spell without paying its mana cost (…)?", then the part (asked when both
/// the creature and its adventure fit)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.item.x;
    let hand = game.player(you).hand.clone();
    let castable: Vec<ObjId> = hand.iter().copied().filter(|c| parts(game, you, *c, n) != (false, false)).collect();
    let card = match castable.len() {
        0 => return Some(()),
        1 => castable[0],
        _ => match choose_cards(game, d, you, &castable, &castable, 0, 1, "cast for free")?.first() {
            Some(c) => *c,
            None => return Some(()),
        },
    };
    let (main, adventure) = parts(game, you, card, n);
    let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
    if !choose_use(game, d, you, &message)? {
        return Some(());
    }
    let before = game.clone();
    let pick_adventure = match (main, adventure) {
        (true, true) => match ask(game, d, you, &Query::ChooseAbility { source: card, abilities: vec![0, 1] }) {
            Answer::Ability(o, 0) if o == card => true,
            Answer::Stop => return None,
            _ => false,
        },
        (false, true) => true,
        _ => false,
    };
    if pick_adventure {
        let full = game.obj(card).def.clone();
        if let Some(a) = full.adventure.clone() {
            let o = game.obj_mut(card);
            o.adventurer = Some(full);
            o.def = a;
        }
    }
    if !cast_free(game, d, you, card) {
        *game = before;
    }
    if d.stopped() {
        return None;
    }
    Some(())
}
