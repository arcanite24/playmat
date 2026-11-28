//! Regression tests for AI multi-blocking against lethal Trample damage.

use playmat_ai::combat;
use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::policy::Policy;
use playmat_engine::decide::{Answer, Query};
use playmat_engine::state::{Game, ObjId, Zone};

fn setup_trample_vs_tokens(defender_life: i32, token_count: usize) -> (Game, ObjId, Vec<ObjId>) {
    let db = playmat_engine::cards();
    let atk_deck = Deck::parse("atk", "20 Forest\n40 Colossal Dreadmaw");
    let def_deck = Deck::parse("def", "20 Forest\n40 Llanowar Elves");
    let mut game = new_duel(&db, [&atk_deck, &def_deck], 42, 0).unwrap();

    // Put one Colossal Dreadmaw (6/6 Trample) on battlefield for seat 0 and mark it attacking seat 1
    let dreadmaw = game
        .player(0)
        .library
        .iter()
        .chain(game.player(0).hand.iter())
        .copied()
        .find(|id| game.obj(*id).def.name == "Colossal Dreadmaw")
        .expect("Colossal Dreadmaw in deck");
    game.move_to(dreadmaw, Zone::Battlefield);
    game.obj_mut(dreadmaw).attacking = Some(1);

    // Put `token_count` 1/1 Llanowar Elves on battlefield for seat 1
    let elves: Vec<ObjId> = game
        .player(1)
        .library
        .iter()
        .chain(game.player(1).hand.iter())
        .copied()
        .filter(|id| game.obj(*id).def.name == "Llanowar Elves")
        .take(token_count)
        .collect();
    assert_eq!(elves.len(), token_count);
    for id in &elves {
        game.move_to(*id, Zone::Battlefield);
    }

    game.player_mut(1).life = defender_life;
    (game, dreadmaw, elves)
}

#[test]
fn policy_gang_blocks_lethal_trample_with_tokens() {
    // 6/6 trample attacking into 3 life with six 1/1s: must block with at least 4 to survive at 1 life.
    let (mut game, dreadmaw, elves) = setup_trample_vs_tokens(3, 6);
    let mut policy = Policy::version(2);

    let mut assigned = Vec::new();
    for _ in 0..10 {
        let q = Query::Blockers { creatures: elves.clone() };
        match policy.answer(&game, 1, &q) {
            Answer::Obj(blocker) => {
                assert!(!assigned.contains(&blocker), "duplicate blocker picked");
                assigned.push(blocker);
                game.obj_mut(blocker).blocking.push(dreadmaw);
                game.obj_mut(dreadmaw).blocked = true;
                game.obj_mut(dreadmaw).blocked_by.push(blocker);
            }
            Answer::Bool(true) => break,
            other => panic!("unexpected blocker answer: {other:?}"),
        }
    }

    assert_eq!(
        assigned.len(),
        4,
        "AI should commit 4 1/1s against a 6/6 trampler at 3 life (takes 2, survives at 1), got {}",
        assigned.len()
    );
}

#[test]
fn combat_plans_includes_multi_block_plan_on_large_token_board() {
    // 6 blockers => 2^6 = 64 > MAX_PLANS * 2 (48), triggering the large-board fallback in combat::plans.
    let (game, _dreadmaw, elves) = setup_trample_vs_tokens(3, 6);
    let q = Query::Blockers { creatures: elves.clone() };
    let plans = combat::plans(&game, &q, &Answer::Bool(true)).expect("blocker plans");

    let has_surviving_multi_block = plans.iter().any(|p| {
        let blocker_picks = p.iter().filter(|a| matches!(a, Answer::Obj(_))).count();
        blocker_picks >= 4
    });
    assert!(
        has_surviving_multi_block,
        "combat::plans must include a multi-blocker plan when tokens are needed to stop lethal trample"
    );
}

#[test]
fn policy_does_not_chump_non_lethal_trample() {
    // At 20 life against a 6/6 trampler, the AI should keep its 1/1s rather than throwing them away.
    let (game, _dreadmaw, elves) = setup_trample_vs_tokens(20, 6);
    let mut policy = Policy::version(2);
    let q = Query::Blockers { creatures: elves };
    assert_eq!(policy.answer(&game, 1, &q), Answer::Bool(true));
}

#[test]
fn commander_playbook_profiles_resolve() {
    let expected = [
        "cmdr_voltron",
        "cmdr_tokens",
        "cmdr_ramp",
        "cmdr_aristocrats",
        "cmdr_spellslinger",
        "cmdr_tribal",
        "cmdr_reanimator",
        "cmdr_lifegain",
        "cmdr_artifacts",
        "cmdr_control",
    ];
    for id in expected {
        let spec = format!("profile:{id}");
        let pb = playmat_ai::playbook::resolve(Some(&spec))
            .unwrap_or_else(|| panic!("expected commander playbook profile {spec} to resolve"));
        assert!(pb.protect_roles.iter().any(|r| r == "commander"), "profile {id} should protect commander role");
    }
}
