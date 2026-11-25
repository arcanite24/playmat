//! Nissa, Sage Animist (Nissa, Vastwood Seer's back face): "+1: Reveal the top card of your library. If it's a land
//! card, put it onto the battlefield. Otherwise, put it into your hand. −7: Untap up to six target lands. They become
//! 6/6 Elemental creatures. They're still lands."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("NissaVastwoodSeer.NissaSageAnimistPlusOneEffect", plus_one),
    Handler::effect("NissaVastwoodSeer.NissaSageAnimistMinusAnimateEffect", animate).effects(&[
        "new BecomesCreatureTargetEffect(new CreatureToken(6,6,\"\",SubType.ELEMENTAL),false,true,Duration.Custom)",
    ]),
];

/// the top card revealed, then onto the battlefield if it's a land card, else into the hand (nothing with an empty
/// library)
fn plus_one(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let card = *game.player(you).library.first()?;
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    if game.obj(card).def.types.contains(&CardType::Land) {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
            return None;
        }
    } else {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}

/// each target land still there becomes a 6/6 Elemental creature that's still a land, for good (`Duration.Custom`,
/// one effect per land)
fn animate(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield {
            cx.run_on(game, d, cx.effects, vec![Target::Obj(o)]);
        }
    }
    (!d.stopped()).then_some(())
}
