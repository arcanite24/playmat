//! Remorseless Punishment: "Target opponent loses 5 life unless that player discards two cards or sacrifices a
//! creature or planeswalker. Repeat this process once."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RemorselessPunishment.RemorselessPunishmentEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opp) = cx.player(0) else { return Some(()) };
    punish(game, d, cx, opp, "1st")?;
    punish(game, d, cx, opp, "2nd")
}

/// `handleBaseEffect`: with two or more cards in hand, "discard two cards?"; else (or declined) with a creature or
/// planeswalker, "sacrifice one?" and one chosen sacrificed; else 5 life lost
fn punish(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, opp: Seat, nth: &str) -> Option<()> {
    let message = format!("Choose your {} punishment.", nth);
    if game.player(opp).hand.len() > 1 && choose_use(game, d, opp, &message)? {
        discard(game, d, opp, 2);
        return (!d.stopped()).then_some(());
    }
    let victims = |game: &Game| -> Vec<ObjId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|o| {
                game.obj(*o).controller == opp
                    && (game.has_type(*o, CardType::Creature) || game.has_type(*o, CardType::Planeswalker))
            })
            .collect()
    };
    if !victims(game).is_empty() && choose_use(game, d, opp, &message)? {
        let possible = victims(game);
        let rule = item_rule(game, cx.item);
        if let Some(&o) = choose_cards(game, d, opp, &possible, &possible, 1, 1, &rule)?.first() {
            if game.obj(o).zone == Zone::Battlefield {
                sacrifice(game, o);
            }
            return Some(());
        }
    }
    lose_life_by(game, opp, 5, Some(cx.source_id()), false);
    Some(())
}
