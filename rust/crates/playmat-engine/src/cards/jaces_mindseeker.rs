//! Jace's Mindseeker: "When Jace's Mindseeker enters, target opponent mills five cards. You may cast an instant or
//! sorcery spell from among them without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JacesMindseeker.JaceMindseekerEffect", apply)];

/// the milled cards (in the graveyard afterwards), then `castSpellWithAttributesForFree` with an instant or sorcery
/// filter: one castable card is taken as it is, more are chosen among (up to one); `chooseUse`, then the cast
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(p).library.iter().take(5).copied().collect();
    cx.run(game, d, &[EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(5) }]);
    if d.stopped() {
        return None;
    }
    let options: Vec<ObjId> = top
        .into_iter()
        .filter(|c| {
            game.obj(*c).zone == Zone::Graveyard
                && (game.obj(*c).is(CardType::Instant) || game.obj(*c).is(CardType::Sorcery))
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
    let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
    if choose_use(game, d, you, &message)? {
        cast_free(game, d, you, card);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
