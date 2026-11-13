//! Widespread Brutality: "Amass Zombies 2, then the Army you amassed deals damage equal to its power to each non-Army
//! creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WidespreadBrutality.WidespreadBrutalityEffect", apply)
    .effects(&["new AmassEffect(2, SubType.ZOMBIE)"])];

/// The amass, then that Army, if it's still there, deals damage equal to its power to each non-Army creature, in
/// battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(army) = amass(game, d, cx, 2)? else { return Some(()) };
    if game.obj(army).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(army).0;
    let victims: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) && !game.has_subtype(*o, "Army"))
        .collect();
    for v in victims {
        if game.obj(v).zone == Zone::Battlefield {
            deal_damage(game, army, Target::Obj(v), power, false);
        }
    }
    Some(())
}

/// `AmassEffect.doAmass(n, …)`, as the engine's `AmassEffect` (`cx.effects[0]` is one, for its subtype and token),
/// keeping the Army chosen: None when the decider stopped, Some(None) with no Army
pub(super) fn amass(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, n: i32) -> Option<Option<ObjId>> {
    let you = cx.controller();
    let EffectDef::AmassEffect { subtype, token, .. } = &cx.effects[0] else { return Some(None) };
    let armies = |g: &Game| -> Vec<ObjId> {
        g.battlefield.iter().copied().filter(|o| g.obj(*o).controller == you && g.has_subtype(*o, "Army")).collect()
    };
    if armies(game).is_empty() {
        let e = EffectDef::CreateTokenEffect {
            token: token.clone(),
            amount: crate::card::Amount::Fixed(1),
            tapped_and_attacking: false,
            tapped: false,
            attacking: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            haste_until_end_of_turn: false,
            also: None,
            attach_source: None,
        };
        cx.run_on(game, d, &[e], Vec::new());
        if d.stopped() {
            return None;
        }
    }
    let found = armies(game);
    let army = match found.len() {
        0 => return Some(None),
        1 => found[0],
        _ => {
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, you, &found, &found, 1, 1, &rule)?.first() {
                Some(a) => *a,
                None => return Some(None),
            }
        }
    };
    if !game.has_subtype(army, subtype) {
        let zcc = game.obj(army).zcc;
        game.subtypes_while_there.push((army, zcc, subtype.clone()));
    }
    if n > 0 {
        put_counters(game, army, "+1/+1", n, you);
    }
    Some(Some(army))
}
