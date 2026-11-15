//! `ReturnFromGraveyardToBattlefieldWithCounterTargetEffect(counter, counter)` (Perennation): "return target
//! permanent card from your graveyard to the battlefield with a hexproof counter and an indestructible counter on it"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnFromGraveyardToBattlefieldWithCounterTargetEffect", apply)
    .args(&["counter", "counter"])];

/// The one-counter form's return (`ReturnFromGraveyardToBattlefieldTargetEffect` with its counters), then the
/// second counter on what came back the same way
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.args.len() != 2 {
        return Some(());
    }
    let mut counters = Vec::new();
    for a in cx.args {
        let (name, n) = a.rsplit_once(' ')?;
        counters.push(crate::card::CounterDef { name: name.to_string(), amount: n.parse().ok()? });
    }
    // (as the engine's one-counter form: none for a double-faced card or an Aura, which the counters don't find)
    let returning: Vec<ObjId> = cx
        .objs(0)
        .into_iter()
        .filter(|o| {
            let ob = game.obj(*o);
            ob.zone == Zone::Graveyard
                && !ob.token
                && ob.def.transform_back.is_none()
                && ob.def.back.is_none()
                && !ob.def.subtypes.iter().any(|s| s == "Aura")
        })
        .collect();
    let back = EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect {
        tapped: false,
        owner: false,
        counters: Some(counters[0].clone()),
        attacking: false,
    };
    cx.run(game, d, &[back]);
    if d.stopped() {
        return None;
    }
    for o in returning {
        if game.obj(o).zone == Zone::Battlefield {
            add_counters_by(game, o, &counters[1].name, counters[1].amount, Some(cx.controller()));
        }
    }
    Some(())
}
