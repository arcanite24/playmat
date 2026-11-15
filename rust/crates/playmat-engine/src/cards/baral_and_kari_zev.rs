//! Baral and Kari Zev: "Whenever you cast your first instant or sorcery spell each turn, you may cast a spell with
//! lesser mana value that shares a card type with it from your hand without paying its mana cost. If you don't,
//! create First Mate Ragavan, a legendary 2/1 red Monkey Pirate creature token. It gains haste until end of turn."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "BaralAndKariZev.BaralAndKariZevTriggeredAbility",
        "new SpellCastControllerTriggeredAbility(new BaralAndKariZevEffect(), StaticFilters.FILTER_SPELL_AN_INSTANT_OR_SORCERY, false)",
        first_instant_or_sorcery,
    ),
    Handler::effect("BaralAndKariZev.BaralAndKariZevEffect", apply)
        .tokens(&["FirstMateRagavanToken"])
        .effects(&["new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.EndOfTurn)"]),
];

fn instant_or_sorcery(game: &Game, c: ObjId) -> bool {
    game.obj(c).is(CardType::Instant) || game.obj(c).is(CardType::Sorcery)
}

/// `SpellsCastWatcher`: the spell this trigger saw is the first instant or sorcery you cast this turn
fn first_instant_or_sorcery(game: &Game, item: &StackItem) -> bool {
    let Some(spell) = item.cast_spell else { return false };
    game.watch.spells.iter().find(|s| s.caster == item.controller && instant_or_sorcery(game, s.card)).map(|s| s.card)
        == Some(spell)
}

/// `castSpellWithAttributesForFree` over the hand with "shares a card type with that spell, lesser mana value" (the
/// spell's mana value as cast is the trigger's X); with nothing cast, the token, and haste for it until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spell = cx.item.cast_spell?;
    let types: Vec<Pred> = game.obj(spell).def.types.iter().map(|t| Pred::Type { t: *t }).collect();
    let filter = Filter::of(vec![Pred::Or { xs: types }, Pred::Mv { cmp: Cmp::FewerThan, n: cx.item.x }]);
    let hand = game.player(you).hand.clone();
    if !cast_one_for_free(game, d, you, cx.source_id(), &filter) {
        return None;
    }
    let cast = hand.iter().any(|c| game.obj(*c).zone == Zone::Stack);
    if cast {
        return Some(());
    }
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, cx.tokens.first()?, 1);
    let made: Vec<Target> =
        game.battlefield.iter().copied().filter(|o| !before.contains(o)).map(Target::Obj).collect();
    if !made.is_empty() {
        cx.run_on(game, d, &cx.effects[..1], made);
    }
    Some(())
}
