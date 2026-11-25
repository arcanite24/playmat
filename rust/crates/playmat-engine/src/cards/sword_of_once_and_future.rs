//! Sword of Once and Future: "… Then you may cast an instant or sorcery spell with mana value 2 or less from your
//! graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SwordOfOnceAndFuture.SwordOfOnceAndFutureCastEffect", apply)];

/// `castSpellWithAttributesForFree` over your graveyard with "instant or sorcery, mana value less than 3": one
/// castable card is taken as it is, more are chosen among (up to one); `chooseUse`, then the cast, exiled instead of
/// put into your graveyard as it leaves the stack (`SwordOfOnceAndFutureReplacementEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let options: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            ((o.is(CardType::Instant) || o.is(CardType::Sorcery)) && o.def.mana_value() < 3)
                || o.def.prepare.as_deref().is_some_and(fits)
        })
        .collect();
    let card = match options.len() {
        0 => return Some(()),
        1 => options[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, you, &options, &options, 0, 1, &rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    // (`getCastableComponents`: the parts that fit, a prepare card's spell among them; its creature is what's cast)
    let mut parts = Vec::new();
    let o = game.obj(card);
    if (o.is(CardType::Instant) || o.is(CardType::Sorcery)) && o.def.mana_value() < 3 {
        parts.push(log::object(game, card));
    }
    if let Some(p) = o.def.prepare.as_deref().filter(|p| fits(p)) {
        let r = crate::view::stack_ref(game, card);
        parts.push(format!("<font color='{}' object_id='{r}f1'>{}</font> [{r}f1]", crate::prompt::log_color(&p.colors), p.name));
    }
    let message = format!("Cast spell without paying its mana cost ({})?", parts.join(" or "));
    if choose_use(game, d, you, &message)? {
        let cast = cast_free(game, d, you, card);
        if d.stopped() {
            return None;
        }
        // (a prepare card's creature resolves onto the battlefield: only a spell going to the graveyard is exiled)
        if cast && !game.obj(card).def.is_permanent_card() {
            game.obj_mut(card).flashback = true;
        }
    }
    Some(())
}

/// A prepare spell that fits the filter: an instant or sorcery with mana value 2 or less
fn fits(p: &CardDef) -> bool {
    (p.is(CardType::Instant) || p.is(CardType::Sorcery)) && p.mana_value() < 3
}
