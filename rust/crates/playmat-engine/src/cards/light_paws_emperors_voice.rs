//! Light-Paws, Emperor's Voice: "Whenever an Aura you control enters, if you cast it, you may search your library
//! for an Aura card with mana value less than or equal to that Aura and with a different name than each Aura you
//! control, put that card onto the battlefield attached to Light-Paws, Emperor's Voice, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LightPawsEmperorsVoice.LightPawsEmperorsVoiceEffect", apply),
    Handler::predicate("LightPawsEmperorsVoice.LightPawsEmperorsVoiceEffectPredicate", unlike_your_auras),
    Handler::predicate("LightPawsEmperorsVoice.LightPawsEmperorsVoicePredicate", was_cast),
];

/// "if you cast it": the permanent entered by resolving as a spell (`getSpellOrLKIStack`, one zone change before)
fn was_cast(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).zone == Zone::Battlefield && game.obj(id).cast
}

/// a card with a different name than each Aura you control (`CardUtil.haveSameNames`)
fn unlike_your_auras(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let name = game.obj(id).name();
    game.battlefield.iter().all(|a| {
        !(game.obj(*a).controller == you
            && game.has_subtype(*a, "Aura")
            && !name.is_empty()
            && game.obj(*a).name() == name)
    })
}

/// "Search for an Aura?"; the search; the card onto the battlefield attached to this, when this is still there and
/// the card's enchant filter takes it (`AuraCardCanAttachToPermanentId`); then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(aura) = cx.item.entering else { return Some(()) };
    if !choose_use(game, d, you, "Search for an Aura?")? {
        return Some(());
    }
    let mv = game.obj(aura).def.mana_value();
    let filter: crate::card::Filter = serde_json::from_value(serde_json::json!([
        {"p": "subtype", "s": "Aura"},
        {"p": "mv", "cmp": "FEWER_THAN", "n": mv + 1},
        {"p": "card", "handler": "LightPawsEmperorsVoice.LightPawsEmperorsVoiceEffectPredicate"}
    ]))
    .ok()?;
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        if cx.source_on_battlefield(game) && can_attach(game, you, card, cx.source_id()) {
            let to = cx.source_id();
            game.attach_entering = Some((card, to, game.obj(to).zcc));
            put_onto_battlefield(game, d, card, you);
            game.attach_entering = None;
            if d.stopped() {
                return None;
            }
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
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
