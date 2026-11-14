//! Runo Stromkirk // Krothuss, Lord of the Deep: "At the beginning of your upkeep, look at the top card of your
//! library. You may reveal that card. If a creature card with mana value 6 or greater is revealed this way,
//! transform Runo Stromkirk." / "Whenever Krothuss, Lord of the Deep attacks, create a tapped and attacking token
//! that's a copy of another target attacking creature. If that creature is a Kraken, Leviathan, Octopus, or
//! Serpent, create two of those tokens instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("RunoStromkirk.RunoStromkirkEffect", upkeep),
    Handler::effect("RunoStromkirk.KrothussLordOfTheDeepEffect", attacks).effects(&[
        "new CreateTokenCopyTargetEffect(null, null, false, 1, true, true)",
        "new CreateTokenCopyTargetEffect(null, null, false, 2, true, true)",
    ]),
];

/// The top card looked at, "Reveal …?", and a creature card of mana value 6 or more revealed transforms the source
fn upkeep(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    let name = game.obj(card).def.name.clone();
    if !choose_use(game, d, you, &format!("Reveal {name}?"))? {
        return Some(());
    }
    let o = game.obj(card);
    if !o.is(CardType::Creature) || o.def.mana_value() < 6 {
        return Some(());
    }
    let src = cx.source_id();
    if cx.source_on_battlefield(game) {
        transform(game, src);
    }
    Some(())
}

/// The target still on the battlefield: one token copy of it, tapped and attacking, or two when it's a Kraken,
/// Leviathan, Octopus or Serpent
fn attacks(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(of) = cx.obj(0) else { return Some(()) };
    if game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let two = ["Kraken", "Leviathan", "Octopus", "Serpent"].iter().any(|s| game.has_subtype(of, s));
    let e = if two { &cx.effects[1..2] } else { &cx.effects[..1] };
    cx.run_on(game, d, e, vec![Target::Obj(of)]);
    if d.stopped() {
        return None;
    }
    Some(())
}
