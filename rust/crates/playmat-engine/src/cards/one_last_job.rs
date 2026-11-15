//! One Last Job: "Spree. + {2} — Return target creature card from your graveyard to the battlefield. + {1} — Return
//! target Mount or Vehicle card from your graveyard to the battlefield. + {1} — Return target Aura or Equipment card
//! from your graveyard to the battlefield attached to a creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OneLastJob.OneLastJobEffect", apply)];

/// You choose a creature you control the card could be attached to (`TargetPermanent`, not targeted,
/// `Player.choose`); the card onto the battlefield attached to it. With none: an Aura stays, an Equipment returns
/// unattached
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let aura = game.obj(card).def.has_subtype("Aura");
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .filter(|o| !aura || can_attach(game, you, card, *o))
        .collect();
    let to = if creatures.is_empty() { None } else { choose_cards_rounds(game, d, you, &creatures, 1, 1)?.first().copied() };
    match to {
        Some(to) if aura => {
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
        None if aura => {}
        None => put_onto_battlefield(game, d, card, you),
    }
    (!d.stopped()).then_some(())
}

/// an Aura card whose spell's target filter matches the permanent (`CanBeEnchantedByPredicate`)
fn can_attach(game: &Game, you: Seat, card: ObjId, to: ObjId) -> bool {
    game.obj(card).def.spell.targets.iter().any(|t| match t {
        TargetDef::TargetPermanent { filter, .. } => game.filter_matches(filter, you, Some(card), to),
        t => possible_targets(game, you, t, Some(card)).1.contains(&to),
    })
}
