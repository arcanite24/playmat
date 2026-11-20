package mage.cards.repository;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import org.junit.jupiter.api.Test;

/**
 * What players are told when the card database fails: a full disk says so, anything else is a database problem.
 * Never "memory issues" (2026-11-20: a full disk was reported as one until the server was restarted).
 */
public class CardRepositoryProblemTest {

    @Test
    public void aFullDiskIsCalledFull() {
        String problem = CardRepository.describeProblem(100L << 20, 75L << 30);
        assertTrue(problem.contains("disk is full"), problem);
        assertTrue(problem.contains("100 MB free"), problem);
    }

    @Test
    public void otherwiseItsTheDatabase() {
        String problem = CardRepository.describeProblem(20L << 30, 75L << 30);
        assertTrue(problem.contains("card database isn't answering"), problem);
        assertFalse(problem.contains("memory"));
    }

    @Test
    public void reopensNeverStopForGood() {
        long now = 1_000_000_000L;
        assertTrue(CardRepository.mayReopen(0, now, now), "the first tries go at once");
        assertTrue(CardRepository.mayReopen(9, now, now));
        assertFalse(CardRepository.mayReopen(10, now - 1000, now), "past the limit: not every request");
        assertTrue(CardRepository.mayReopen(10, now - CardRepository.RETRY_AFTER_MS, now), "but once a minute, for ever");
        assertTrue(CardRepository.mayReopen(500, now - CardRepository.RETRY_AFTER_MS, now));
    }

    @Test
    public void anUnknownDiskIsTheDatabase() {
        assertTrue(CardRepository.describeProblem(0, 0).contains("isn't answering"));
    }
}
