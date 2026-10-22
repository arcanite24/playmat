package mage.server.career;

import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The solo trainers on the Career side: assisted games pay half (S010, S011), draft picks are graded with signals
 * (S019), the pool coach ranks colour pairs and builds 40 cards (S020), and the stack drills' objectives check out (S021).
 */
public class CareerTrainersTest {

    private static CareerLimited.CareerPoolCard card(String name, String colors, String rarity, int rating, boolean creature, int manaValue) {
        CareerLimited.CareerPoolCard card = new CareerLimited.CareerPoolCard();
        card.name = name;
        card.setCode = "TST";
        card.cardNumber = name.replaceAll("[^0-9]", "");
        card.rarity = rarity;
        card.colors = colors;
        card.rating = rating;
        card.creature = creature;
        card.manaValue = manaValue;
        return card;
    }

    // ---- assists

    @Test
    public void anAssistedGamePaysHalfAndSaysWhy() {
        CareerRules.Reward reward = new CareerRules.Reward(100, 60, null);
        assertSame(reward, CareerService.assistedReward(reward, null), "an unassisted game pays in full");
        CareerRules.Reward half = CareerService.assistedReward(reward, "a hint");
        assertEquals(50, half.coins);
        assertEquals(30, half.xp);
        assertTrue(half.note.contains("a hint") && half.note.contains("half"), half.note);
    }

    @Test
    public void puzzlesAndTrialsTakeNoAssists() {
        CareerService.CareerTable duel = new CareerService.CareerTable("Ana", "roster-1", 1, "duel", null, null);
        assertTrue(duel.allowsAssists());
        assertFalse(new CareerService.CareerTable("Ana", "puzzle:first-bolt", 1, "puzzle", "first-bolt", null).allowsAssists());
        assertFalse(new CareerService.CareerTable("Ana", "trial:five-paths/w", 1, "trial", "five-paths/w", null).allowsAssists());
    }

    // ---- the draft trainer

    @Test
    public void picksAreGradedAgainstTheBestCardForThePool() {
        List<CareerLimited.CareerPoolCard> pack = new ArrayList<>(Arrays.asList(
                card("Bomb 1", "R", CareerRules.RARE, 120, true, 4),
                card("Fine 2", "R", CareerRules.COMMON, 100, true, 2),
                card("Filler 3", "R", CareerRules.COMMON, 50, false, 3)));
        CareerLimited.PickRecord best = CareerLimited.grade(pack, 0, new ArrayList<>(), 1, 1);
        assertEquals("A", best.grade);
        assertEquals("Bomb 1", best.best);
        CareerLimited.PickRecord fine = CareerLimited.grade(pack, 1, new ArrayList<>(), 1, 1);
        assertEquals("C", fine.grade, "100 of 120 is a C");
        assertTrue(fine.note.startsWith("Bomb 1 was stronger"), fine.note);
        assertEquals("F", CareerLimited.grade(pack, 2, new ArrayList<>(), 1, 1).grade);

        // with a blue-green pool, the red bomb is worth less and a good blue card is the pick
        List<CareerLimited.CareerPoolCard> pool = new ArrayList<>();
        for (int i = 0; i < 6; i++) {
            pool.add(card("Blue " + i, i % 2 == 0 ? "U" : "G", CareerRules.COMMON, 90, true, 2));
        }
        pack.add(card("Blue pick 4", "U", CareerRules.UNCOMMON, 100, true, 3));
        CareerLimited.PickRecord onColour = CareerLimited.grade(pack, 3, pool, 2, 4);
        assertEquals("A", onColour.grade, onColour.note);
    }

    @Test
    public void signalsShowTheColourThatKeepsComingLate() {
        Map<String, Double> passed = new LinkedHashMap<>();
        List<CareerLimited.CareerPoolCard> early = Arrays.asList(card("Red 1", "R", CareerRules.RARE, 130, true, 3));
        CareerLimited.notePassed(passed, early, 1);
        assertTrue(passed.isEmpty(), "the first picks of a pack say nothing");
        for (int pick = 3; pick <= 8; pick++) {
            CareerLimited.notePassed(passed, Arrays.asList(
                    card("Green " + pick, "G", CareerRules.UNCOMMON, 110, true, 3),
                    card("White " + pick, "W", CareerRules.COMMON, 50, true, 2)), pick);
        }
        List<CareerLimited.ColorSignal> signals = CareerLimited.signals(passed);
        assertEquals("G", signals.get(0).color);
        assertEquals(100, signals.get(0).strength);
        assertEquals(0, signals.get(signals.size() - 1).strength, "nothing blue, black or red came");
    }

    // ---- the pool coach

    @Test
    public void theCoachRanksPairsWithReasonsAndBuildsForty() {
        List<CareerLimited.CareerPoolCard> pool = new ArrayList<>();
        for (int i = 0; i < 14; i++) {
            CareerLimited.CareerPoolCard black = card("Black " + i, "B", CareerRules.COMMON, 95, i < 10, 2 + i % 4);
            black.removal = i >= 11;
            pool.add(black);
            pool.add(card("Red " + i, "R", CareerRules.COMMON, 90, true, 1 + i % 5));
            pool.add(card("White " + i, "W", CareerRules.COMMON, 40, true, 3));
        }
        pool.add(card("Dragon 99", "R", CareerRules.MYTHIC, 140, true, 5));
        CareerLimited.CareerLimitedCoach coach = CareerLimited.coach(pool);
        assertEquals(10, coach.pairs.size(), "every colour pair");
        CareerLimited.PairAdvice best = coach.pairs.get(0);
        assertEquals("BR", best.colors);
        assertEquals(3, best.removal);
        assertEquals(Arrays.asList("Dragon 99"), best.bombs);
        assertTrue(best.reasons.stream().anyMatch(reason -> reason.startsWith("Bombs: Dragon 99")), best.reasons.toString());
        assertTrue(best.reasons.stream().anyMatch(reason -> reason.startsWith("3 removal spells")), best.reasons.toString());
        int size = best.deck.stream().mapToInt(info -> info.getAmount()).sum();
        assertEquals(CareerLimited.DECK_SIZE, size, "the build is 40 cards");
        assertTrue(best.deck.stream().anyMatch(info -> "Swamp".equals(info.getCardName())));
        assertTrue(best.deck.stream().anyMatch(info -> "Mountain".equals(info.getCardName())));
        assertTrue(best.deck.stream().noneMatch(info -> info.getCardName().startsWith("White")));
    }

    // ---- the stack drills

    @Test
    public void stackDrillsOpenAsTheirOwnSeriesAndCheckTheirObjectives() throws Exception {
        List<CareerSetup.Puzzle> drills = new ArrayList<>();
        for (CareerSetup.Puzzle puzzle : CareerPuzzles.readBuiltIn()) {
            assertNull(puzzle.problem(), puzzle.id);
            if ("Stack and priority".equals(puzzle.series)) {
                drills.add(puzzle);
            }
        }
        assertEquals(5, drills.size());
        assertTrue(drills.stream().anyMatch(puzzle -> "protect".equals(puzzle.objective) && "Serra Angel".equals(puzzle.protect)));
        assertTrue(drills.stream().anyMatch(puzzle -> "you".equals(puzzle.firstTurn) && puzzle.survive()));

        CareerSetup.Puzzle broken = new CareerSetup.Puzzle();
        broken.objective = "protect";
        broken.protect = "Nothing";
        broken.you = new CareerSetup.CareerPuzzleSide();
        broken.you.battlefield = Arrays.asList("Grizzly Bears");
        assertNotNull(broken.problem(), "a protect objective names a permanent you have");
        broken.protect = "Grizzly Bears";
        assertNull(broken.problem());
        assertTrue(broken.opponentFirst(), "a protect trial starts on the opponent's turn");
        broken.objective = "win";
        broken.firstTurn = "opponent";
        assertNotNull(broken.problem(), "a win-this-turn puzzle starts on your turn");
    }
}
