//! Nissa, Worldwaker: "+1: Target land you control becomes a 4/4 Elemental creature with trample. It's still a
//! land. +1: Untap up to four target Forests. −7: Search your library for any number of basic land cards, put them
//! onto the battlefield, then shuffle. Those lands become 4/4 Elemental creatures with trample. They're still
//! lands."

use super::*;
use crate::card::{AbilityDef, Duration, Pred, TokenDef};

pub const HANDLERS: &[Handler] = &[Handler::effect("NissaWorldwaker.NissaWorldwakerSearchEffect", apply)];

/// any number of basic land cards found; each put onto the battlefield on its own, then made a 4/4 Elemental with
/// trample for as long as it stays (`Duration.Custom`, still a land); the shuffle last
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = Filter(vec![Pred::Type { t: CardType::Land }, Pred::Supertype { s: "BASIC".into() }]);
    let found = search_library(game, d, you, &filter, 0, usize::MAX, cx.source_id())?;
    let token = TokenDef {
        types: vec![CardType::Creature],
        subtypes: vec!["Elemental".into()],
        power: 4,
        toughness: 4,
        abilities: vec![AbilityDef::TrampleAbility],
        ..Default::default()
    };
    for c in found {
        if game.obj(c).zone != Zone::Library {
            continue;
        }
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
        if game.obj(c).zone == Zone::Battlefield {
            let e = EffectDef::BecomesCreatureTargetEffect {
                token: token.clone(),
                duration: Duration::WhileOnBattlefield,
                lose_abilities: false,
                still_land: true,
                named: None,
            };
            cx.run_on(game, d, &[e], vec![Target::Obj(c)]);
        }
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}
