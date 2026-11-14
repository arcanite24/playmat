//! Wake to Slaughter: "Choose up to two target creature cards in your graveyard. An opponent chooses one of them.
//! Return that card to your hand. Return the other to the battlefield under your control. It gains haste. Exile it at
//! the beginning of the next end step. Flashback {4}{B}{R}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WakeToSlaughter.WakeToSlaughterEffect", apply)];

/// The targets still in the graveyard. One: it's the random pick for the hand. Two: the opponent (the only one, or
/// the one you choose) picks the one for the hand. Then in target order: that one to your hand, the other onto the
/// battlefield with haste for the game and a delayed "exile it" at the next end step.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let picked: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    if picked.is_empty() {
        return Some(());
    }
    let to_hand = if picked.len() == 1 {
        let i = game.rng.next_int_bound(1) as usize;
        Some(picked[i])
    } else {
        let opponents: Vec<Seat> = (1..game.seats())
            .map(|i| (you + i) % game.seats())
            .filter(|p| !game.player(*p).lost)
            .collect();
        let opponent = match opponents[..] {
            [p] => p,
            _ => {
                // `TargetOpponent(true)` through `chooseTarget`: several opponents aren't played yet
                return Some(());
            }
        };
        let rule = item_rule(game, cx.item);
        choose_cards(game, d, opponent, &picked, &picked, 1, 1, &rule)?.first().copied()
    };
    for c in picked {
        if Some(c) == to_hand {
            put_cards(game, d, you, &[c], crate::card::PutCards::Hand);
            continue;
        }
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
        let zcc = game.obj(c).zcc;
        Arc::make_mut(&mut game.grants).push(crate::state::Grant {
            target: c,
            zcc,
            ability: AbilityDef::HasteAbility,
            until_end_of_turn: false,
            until_turn_of: None,
        });
        let delayed = Delayed {
            rule: "At the beginning of the next end step, exile that permanent.".into(),
            sacrifice: Vec::new(),
            warp_exile: Vec::new(),
            effects: vec![EffectDef::ExileTargetEffect],
            source_zcc: cx.source.1,
            fixed: vec![(c, zcc)],
            return_tapped: Vec::new(),
            targets: Vec::new(),
            condition: None,
        };
        game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    }
    Some(())
}
