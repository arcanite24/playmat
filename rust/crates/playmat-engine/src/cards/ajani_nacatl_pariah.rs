//! Ajani, Nacatl Pariah // Ajani, Nacatl Avenger:
//! "0: Create a 2/1 white Cat Warrior creature token. When you do, if you control a red permanent other than Ajani,
//! Nacatl Avenger, he deals damage equal to the number of creatures you control to any target."
//! "-4: Each opponent chooses an artifact, a creature, an enchantment and a planeswalker from among the nonland
//! permanents they control, then sacrifices the rest."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AjaniNacatlPariah.AjaniNacatlAvengerZeroEffect", zero)
        .effects(&["new CreateTokenEffect(new CatWarrior21Token())"]),
    Handler::effect("AjaniNacatlPariah.AjaniNacatlAvengerMinusFourEffect", minus_four),
];

const TYPES: [CardType; 4] = [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Planeswalker];

fn zero(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.objs.len();
    let pending_before = game.pending.len();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let created = (before..game.objs.len()).any(|i| {
        let o = game.obj(ObjId(i as u32));
        o.token && o.zone == Zone::Battlefield
    });
    if !created {
        return Some(());
    }
    let cond = crate::card::Condition::Permanents {
        filter: Filter(vec![
            Pred::Color { c: Color::R },
            Pred::Another,
            Pred::Controller { who: "you".into() },
        ]),
        cmp: crate::card::Cmp::OrGreater,
        n: 1,
        all: false,
    };
    let dmg = EffectDef::DamageTargetEffect {
        amount: Amount::Of(crate::card::Value::Permanents {
            filter: Filter(vec![
                Pred::Type { t: CardType::Creature },
                Pred::Controller { who: "you".into() },
            ]),
        }),
    };
    let target = TargetDef::TargetAnyTarget {
        min: 1,
        max: 1,
        another: false,
        not_first: false,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "When you do, if you control a red permanent other than {this}, he deals damage equal to the number of creatures you control to any target..".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![dmg],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: Some(cond),
    };
    // Java's `CreateTokenEffect` queues ETB as a simultaneous event, whereas `fireReflexiveTriggeredAbility` fires
    // `OPTION_USED` synchronously before `handleSimultaneousEvent`, so the reflexive trigger waits before any ETB
    // triggers caused by the token.
    let at = pending_before.min(game.pending.len());
    game.pending.insert(at, StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

fn minus_four(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut keep: Vec<ObjId> = Vec::new();
    let rule = item_rule(game, cx.item);
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        for t in TYPES {
            let options: Vec<ObjId> = game
                .battlefield
                .iter()
                .copied()
                .filter(|o| game.obj(*o).controller == p && !game.has_type(*o, CardType::Land) && game.has_type(*o, t))
                .collect();
            if options.is_empty() {
                continue;
            }
            let chosen = choose_cards(game, d, p, &options, &options, 1, 1, &rule)?;
            keep.extend(chosen.first().copied());
        }
    }
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller != you && !game.has_type(*o, CardType::Land) && !keep.contains(o))
        .collect();
    for o in doomed {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}
