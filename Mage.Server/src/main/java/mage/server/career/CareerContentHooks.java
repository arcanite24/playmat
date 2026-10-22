package mage.server.career;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.util.ArrayList;
import java.util.List;

/**
 * What content packs need from the Career content: the ids the server's own content uses (packs can't reuse them),
 * and a way to read everything again once the packs change.
 */
public final class CareerContentHooks {

    private CareerContentHooks() {
    }

    public static List<String> builtInCampaignIds() {
        try {
            return new ArrayList<>(CareerCampaigns.readBuiltIn().keySet());
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    public static List<String> builtInPuzzleIds() {
        try {
            List<String> ids = new ArrayList<>();
            CareerCampaignsPuzzles.ids(CareerPuzzles.readBuiltIn(), ids);
            return ids;
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    public static List<String> builtInChallengeIds() {
        try {
            List<String> ids = new ArrayList<>();
            for (CareerChallenges.Challenge challenge : CareerChallenges.readBuiltIn()) {
                ids.add(challenge.id);
            }
            return ids;
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    public static List<String> builtInOpponentIds() {
        try {
            return CareerContent.builtInOpponentIds();
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    public static List<Integer> tiers() {
        try {
            List<Integer> tiers = new ArrayList<>();
            for (CareerContent.Tier tier : CareerContent.builtInTiers()) {
                tiers.add(tier.tier);
            }
            return tiers;
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    /** the packs changed: every content list is read again on its next use */
    public static void reloadAll() {
        CareerContent.forget();
        CareerCampaigns.forget();
        CareerPuzzles.forget();
        CareerChallenges.forget();
    }

    private static final class CareerCampaignsPuzzles {
        static void ids(List<CareerSetup.Puzzle> puzzles, List<String> ids) {
            for (CareerSetup.Puzzle puzzle : puzzles) {
                ids.add(puzzle.id);
            }
        }
    }
}
