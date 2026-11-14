//! Taster of Wares: "When this creature enters, target opponent reveals X cards from their hand, where X is the
//! number of Goblins you control. You choose one of those cards. That player exiles it. If an instant or sorcery card
//! is exiled this way, you may cast it for as long as you control this creature, and mana of any type can be spent to
//! cast that spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TasterOfWares.TasterOfWaresEffect", apply)];

/// Nothing with an empty hand or no Goblins; the opponent chooses the X cards (`TargetCardInHand(min(hand, X))`), you
/// choose one of them (one is taken as it is), it's exiled; an instant or sorcery may be cast while you control this
/// creature (`makeCardPlayable(…, Duration.WhileControlled, true)`), only if it's still yours on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let goblins = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Goblin"))
        .count();
    if goblins == 0 {
        return Some(());
    }
    let n = goblins.min(hand.len());
    let rule = item_rule(game, cx.item);
    let revealed = choose_cards(game, d, p, &hand, &hand, n, n, &rule)?;
    let card = match revealed.len() {
        0 => return Some(()),
        1 => revealed[0],
        _ => match choose_cards(game, d, you, &revealed, &revealed, 1, 1, &rule)?.first() {
            Some(c) => *c,
            None => return Some(()),
        },
    };
    game.move_to(card, Zone::Exile);
    let o = game.obj(card);
    if o.zone != Zone::Exile || !(o.is(CardType::Instant) || o.is(CardType::Sorcery)) {
        return Some(());
    }
    let (src, zcc) = cx.source;
    if cx.source_on_battlefield(game) && game.obj(src).controller == you {
        let o = game.obj_mut(card);
        o.may_play = Some((you, u32::MAX));
        o.may_play_end_step = false;
        o.may_play_while = Some((src, zcc));
        o.any_mana = true;
    }
    Some(())
}
