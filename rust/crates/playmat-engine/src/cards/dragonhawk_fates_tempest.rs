//! Dragonhawk, Fate's Tempest: "Whenever Dragonhawk enters or attacks, exile the top X cards of your library, where X
//! is the number of creatures you control with power 4 or greater. You may play those cards until your next end
//! step. At the beginning of your next end step, Dragonhawk deals 2 damage to each opponent for each of those cards
//! that are still exiled."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("DragonhawkFatesTempest.DragonhawkExileEffect", exile),
    Handler::effect("DragonhawkFatesTempest.DragonhawkFatesTempestDamageEffect", damage),
];

const DAMAGE: &str = "DragonhawkFatesTempest.DragonhawkFatesTempestDamageEffect";

/// `ExileTopXMayPlayUntilEffect` for X, then (when any were exiled) an "at the beginning of your next end step" trigger
/// that remembers them
fn exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game
        .battlefield
        .iter()
        .filter(|o| {
            game.obj(**o).controller == you && game.has_type(**o, CardType::Creature) && game.power_toughness(**o).0 >= 4
        })
        .count() as i32;
    let top: Vec<ObjId> = game.player(you).library.iter().take(x.max(0) as usize).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let e = EffectDef::ExileTopXMayPlayUntilEffect {
        amount: Amount::Fixed(x),
        choose_one: false,
        duration: "UntilYourNextEndStep".into(),
    };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    let exiled: Vec<(ObjId, u32)> =
        top.iter().filter(|c| game.obj(**c).zone == Zone::Exile).map(|c| (*c, game.obj(*c).zcc)).collect();
    if exiled.is_empty() {
        return Some(());
    }
    // the cards are the effect's own (`DragonhawkFatesTempestDamageEffect.cards`), not the trigger's target pointer: two
    // of these triggers read the same and are ordered without asking
    let cards: Vec<String> = exiled.iter().map(|(o, z)| format!("{} {}", o.0, z)).collect();
    let effect: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": DAMAGE, "args": cards})).ok()?;
    let delayed = Delayed {
        rule: "At the beginning of your next end step, {this} deals 2 damage to each opponent for each of those cards \
               that are still exiled."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![effect],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_your_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// 2 damage to each opponent for each remembered card (`id zcc` in the args) still in exile as the same object
fn damage(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx
        .args
        .iter()
        .filter_map(|a| {
            let (o, z) = a.split_once(' ')?;
            Some((ObjId(o.parse().ok()?), z.parse::<u32>().ok()?))
        })
        .filter(|(c, z)| game.obj(*c).zone == Zone::Exile && game.obj(*c).zcc == *z)
        .count() as i32;
    if n < 1 {
        return Some(());
    }
    let opponent = game.opponent(cx.controller());
    deal_damage(game, cx.source_id(), Target::Player(opponent), 2 * n, false);
    Some(())
}
