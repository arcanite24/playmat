//! Sin, Spira's Punishment: "Whenever Sin enters or attacks, exile a permanent card from your graveyard at random,
//! then create a tapped token that's a copy of that card. If the exiled card is a land card, repeat this process."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SinSpirasPunishment.SinSpirasPunishmentEffect", apply)];

fn permanent_card(game: &Game, c: ObjId) -> bool {
    let o = game.obj(c);
    [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Land, CardType::Planeswalker, CardType::Battle]
        .iter()
        .any(|t| o.is(*t))
}

/// `RandomUtil.randomFromCollection` over the permanent cards in your graveyard (in its order; no roll for one),
/// exiled, then a tapped token copy of it (`CreateTokenCopyTargetEffect(you, null, false, 1, true, false)`), again
/// while the card exiled is a land card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    loop {
        let cards: Vec<ObjId> =
            game.player(you).graveyard.iter().copied().filter(|c| permanent_card(game, *c)).collect();
        let card = match cards.len() {
            0 => return Some(()),
            1 => cards[0],
            n => cards[game.rng.next_int_bound(n as i32) as usize],
        };
        game.move_to(card, Zone::Exile);
        let made = token_copy_modified(game, d, card, you, 1, &|_: &mut CardDef| {});
        if d.stopped() {
            return None;
        }
        for t in made {
            game.obj_mut(t).tapped = true;
        }
        if !game.obj(card).is(CardType::Land) {
            return Some(());
        }
    }
}
