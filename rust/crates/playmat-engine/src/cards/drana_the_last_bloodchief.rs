//! Drana, the Last Bloodchief: "Flying. Whenever Drana attacks, defending player chooses a nonlegendary creature card
//! in your graveyard. You return that card to the battlefield with an additional +1/+1 counter on it. The creature
//! is a Vampire in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DranaTheLastBloodchief.DranaTheLastBloodchiefEffect", apply)];

/// With a nonlegendary creature card in your graveyard, the defending player (the pointer) chooses one (not
/// targeted); it enters with a +1/+1 counter, under your control, and is a Vampire for as long as it stays
/// (`DranaTheLastBloodchiefSubtypeEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(defender) = cx.player(0) else { return Some(()) };
    let shown = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> = shown
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Creature) && !game.obj(*c).def.supertypes.iter().any(|s| s == "LEGENDARY"))
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, defender, &shown, &possible, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    let zcc = game.obj(card).zcc;
    game.enter_counters.push((card, zcc, 1));
    let back =
        EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect { tapped: false, owner: false, counters: None, attacking: false };
    cx.run_on(game, d, &[back], vec![Target::Obj(card)]);
    game.enter_counters.retain(|(o, z, _)| !(*o == card && *z == zcc));
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        let zcc = game.obj(card).zcc;
        game.subtypes_while_there.push((card, zcc, "Vampire".into()));
    }
    Some(())
}
