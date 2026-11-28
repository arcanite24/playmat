//! Devout Invocation: "Tap any number of untapped creatures you control. Create a 4/4 white Angel creature token
//! with flying for each creature tapped this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DevoutInvocation.DevoutInvocationEffect", apply)
    .effects(&["new CreateTokenEffect(new AngelToken())"])];

/// `TargetControlledPermanent(0, any, untapped creatures, notTarget)`: chosen one at a time until done
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let can: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && !game.obj(*o).tapped)
        .collect();
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        // asked even with nothing to choose from (done is the answer), but not once every one is chosen
        let left: Vec<ObjId> = can.iter().copied().filter(|c| !chosen.contains(c)).collect();
        if left.is_empty() && !chosen.is_empty() {
            break;
        }
        match ask(game, d, you, &Query::Cards { objs: left.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if left.contains(&o) => chosen.push(o),
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ => break,
        }
        if d.stopped() {
            return None;
        }
    }
    let mut tapped = 0;
    for o in chosen {
        if !game.obj(o).tapped {
            tap(game, o);
            tapped += 1;
        }
    }
    if tapped > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
            *amount = Amount::Fixed(tapped);
        }
        cx.run(game, d, &[e]);
    }
    Some(())
}
