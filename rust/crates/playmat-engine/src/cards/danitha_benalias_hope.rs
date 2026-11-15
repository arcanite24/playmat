//! Danitha, Benalia's Hope: "First strike, vigilance, lifelink. When Danitha, Benalia's Hope enters, you may put an
//! Aura or Equipment card from your hand or graveyard onto the battlefield attached to Danitha."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DanithaBenaliasHope.DanithaBenaliasHopeEffect", apply)];

/// The Aura cards that could enchant Danitha (`AuraCardCanAttachToPermanentId`, with Danitha still there) and the
/// Equipment cards in your hand, then your graveyard; `TargetCard(0, 1, …)` chosen with `Player.choose`; the card
/// onto the battlefield, attached to Danitha while it's there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_on_battlefield(game).then(|| cx.source_id());
    let fits = |game: &Game, c: ObjId| {
        game.has_subtype(c, "Equipment")
            || (game.has_subtype(c, "Aura") && me.is_some_and(|to| can_attach(game, you, c, to)))
    };
    let mut cards: Vec<ObjId> = game.player(you).hand.iter().copied().filter(|c| fits(game, *c)).collect();
    cards.extend(game.player(you).graveyard.iter().copied().filter(|c| fits(game, *c)));
    if cards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &cards, &cards, 0, 1, &rule)?.first() else { return Some(()) };
    match me {
        Some(to) if game.has_subtype(card, "Aura") => {
            game.attach_entering = Some((card, to, game.obj(to).zcc));
            put_onto_battlefield(game, d, card, you);
            game.attach_entering = None;
        }
        Some(to) => {
            put_onto_battlefield(game, d, card, you);
            if game.obj(card).zone == Zone::Battlefield && game.obj(to).zone == Zone::Battlefield {
                let zcc = game.obj(to).zcc;
                game.attach_to(card, to, zcc);
            }
        }
        None => put_onto_battlefield(game, d, card, you),
    }
    (!d.stopped()).then_some(())
}

/// an enchantment card whose spell's target filter matches the permanent
fn can_attach(game: &Game, you: Seat, card: ObjId, to: ObjId) -> bool {
    if !game.obj(card).is(CardType::Enchantment) {
        return false;
    }
    game.obj(card).def.spell.targets.iter().any(|t| match t {
        TargetDef::TargetPermanent { filter, .. } => game.filter_matches(filter, you, Some(card), to),
        t => possible_targets(game, you, t, Some(card)).1.contains(&to),
    })
}
