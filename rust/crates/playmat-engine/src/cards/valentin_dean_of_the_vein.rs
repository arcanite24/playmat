//! Valentin, Dean of the Vein // Lisette, Dean of the Root. Valentin: "If a nontoken creature an opponent controls
//! would die, exile it instead. When you do, you may pay {2}. If you do, create a 1/1 black and green Pest creature
//! token with "When this creature dies, you gain 1 life.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "ValentinDeanOfTheVein.ValentinDeanOfTheVeinEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)
.effects(&["new DoIfCostPaid(new CreateTokenEffect(new PestBlackGreenDiesToken()), new GenericManaCost(2))"])];

/// `isDiesEvent` of a nontoken creature controlled by an opponent of Valentin's controller
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    let o = game.obj(id);
    ev.from == Some(Zone::Battlefield)
        && ev.to == Some(Zone::Graveyard)
        && !o.token
        && game.has_type(id, CardType::Creature)
        && o.controller != cx.controller
}

/// Exiled instead, and the reflexive trigger: "you may pay {2}. If you do, create a Pest"
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    let delayed = Delayed {
        rule: "You may pay {2}. If you do, create a 1/1 black and green Pest creature token with \"When this \
               creature dies, you gain 1 life.\"."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: game.obj(cx.source).zcc,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    let item = StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source, cx.controller) };
    // (fired at once, before the dies triggers of a state-based batch: `fireReflexiveTriggeredAbility`)
    match game.simultaneous_from {
        Some(k) if k <= game.pending.len() => {
            game.pending.insert(k, item);
            game.simultaneous_from = Some(k + 1);
        }
        _ => game.pending.push(item),
    }
    false
}
