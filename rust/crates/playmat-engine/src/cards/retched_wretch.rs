//! Retched Wretch: "When this creature dies, if it had a -1/-1 counter on it, return it to the battlefield under its
//! owner's control and it loses all abilities."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("RetchedWretch.RetchedWretchCondition", had_m1m1),
    Handler::effect("RetchedWretch.RetchedWretchReturnEffect", apply),
];

/// the permanent that left (`permanentLeftBattlefield`) had a -1/-1 counter: its counters while it's still on the
/// battlefield (as the trigger is collected), its last known ones after
fn had_m1m1(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    let counters = if o.zone == Zone::Battlefield { &o.counters } else { &o.lki_counters };
    counters.iter().any(|(n, k)| n == "-1/-1" && *k > 0)
}

/// The card, while it's in a graveyard (Java checks the zone only), onto the battlefield under its owner's control;
/// then `LoseAllAbilitiesTargetEffect(Duration.Custom)` on the permanent (as Abigale's: for as long as it stays)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.source_id();
    if game.obj(o).zone != Zone::Graveyard {
        return Some(());
    }
    let owner = game.obj(o).owner;
    put_onto_battlefield(game, d, o, owner);
    if d.stopped() {
        return None;
    }
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let at = game.timestamp();
    let ob = game.obj_mut(o);
    ob.lost_all_at = at;
    if let Some(def) = ob.becomes_base.take() {
        ob.def = def;
    }
    let old = ob.def.clone();
    let mut def = (*old).clone();
    def.abilities.clear();
    ob.def = Arc::new(def);
    if let Some(eot) = ob.until_end_of_turn_def.as_mut() {
        Arc::make_mut(eot).abilities.clear();
    }
    ob.lost_all = 2;
    for t in Arc::make_mut(&mut game.stack).iter_mut().chain(game.pending.iter_mut()) {
        if t.card == o && t.ability.is_some() && t.def.is_none() {
            t.def = Some(old.clone());
        }
    }
    Some(())
}
