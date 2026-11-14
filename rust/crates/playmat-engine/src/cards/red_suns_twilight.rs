//! Red Sun's Twilight: "Destroy up to X target artifacts. If X is 5 or more, for each artifact destroyed this way,
//! create a token that's a copy of it. Those tokens gain haste. Exile them at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RedSunsTwilight.RedSunsTwilightEffect", apply)];

/// Each target destroyed in turn; with X of 5 or more, a token copy with haste of each one destroyed, all exiled
/// at the beginning of the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x;
    let mut destroyed = Vec::new();
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let zcc = game.obj(o).zcc;
        destroy(game, o);
        if game.obj(o).zcc != zcc {
            destroyed.push(o);
        }
    }
    if x < 5 {
        return Some(());
    }
    let mut tokens = Vec::new();
    for o in destroyed {
        let made = token_copy_modified(game, d, o, you, 1, &|def: &mut CardDef| {
            if !def.abilities.contains(&AbilityDef::HasteAbility) {
                def.abilities.push(AbilityDef::HasteAbility);
            }
        });
        if d.stopped() {
            return None;
        }
        tokens.extend(made.into_iter().filter(|t| game.obj(*t).zone == Zone::Battlefield));
    }
    if tokens.is_empty() {
        return Some(());
    }
    let delayed = Delayed {
        rule: "At the beginning of the next end step, exile tokens created with {this}.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: tokens.iter().map(|t| (*t, game.obj(*t).zcc)).collect(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
