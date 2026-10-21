package mage.server.career;

import com.google.gson.Gson;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;

/**
 * What a game outside Career's own tables pays, as data (resources/career/rewards.json): XP by kind of game (against
 * the AI, a person, a pod, an event), the AI's skill, the turn floor, the pair cap, the first win of the day and the
 * daily wins ladder. Career tables keep {@link CareerRules}. The admin's XP and coin settings scale it.
 */
public final class CareerRewards {

    /** the kinds of game progression tells apart */
    public static final String CAREER = "career";
    public static final String AI = "ai";
    public static final String PERSON = "person";
    public static final String POD = "pod";
    public static final String EVENT = "event";
    public static final List<String> KINDS = Collections.unmodifiableList(Arrays.asList(CAREER, AI, PERSON, POD, EVENT));

    public static class Kind {
        public int winXp;
        public int lossXp;
        public int perExtraPlayerXp;
    }

    static class File {
        int turnFloor = CareerRules.MIN_TURN_FOR_REWARD;
        int pairCapPerDay = 5;
        Map<String, Kind> kinds = Collections.emptyMap();
        Map<String, Double> aiSkill = Collections.emptyMap();
        int bo3MatchWinXp;
        int firstWinXp;
        int firstPersonGameXp;
        List<Integer> dailyWinCoins = Collections.emptyList();
        EventPrizes eventPrizes = new EventPrizes();
        int renownXp = 5000;
        List<Integer> masteryRankGames = Collections.emptyList();
        int masteryRankXp;
        int masteryRankCoins;
    }

    static class EventPrizes {
        List<Integer> limitedPacks = Collections.emptyList();
        List<Integer> constructedPacks = Collections.emptyList();
        int xp;
        int xpPerWin;
    }

    private static volatile CareerRewards instance;

    private final File file;

    private CareerRewards(File file) {
        this.file = file;
    }

    public static CareerRewards get() {
        CareerRewards current = instance;
        if (current == null) {
            synchronized (CareerRewards.class) {
                current = instance;
                if (current == null) {
                    current = new CareerRewards(read());
                    instance = current;
                }
            }
        }
        return current;
    }

    private static File read() {
        InputStream stream = CareerRewards.class.getResourceAsStream("/career/rewards.json");
        if (stream == null) {
            throw new IllegalStateException("Missing career resource rewards.json");
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            return new Gson().fromJson(reader, File.class);
        } catch (IOException e) {
            throw new IllegalStateException("rewards.json can't be read: " + e.getMessage(), e);
        }
    }

    public int turnFloor() {
        return file.turnFloor;
    }

    public int pairCapPerDay() {
        return file.pairCapPerDay;
    }

    public int bo3MatchWinXp() {
        return scaleXp(file.bo3MatchWinXp);
    }

    public int firstWinXp() {
        return scaleXp(file.firstWinXp);
    }

    public int firstPersonGameXp() {
        return scaleXp(file.firstPersonGameXp);
    }

    /** coins the day's nth win pays (1-based); 0 past the ladder */
    public int dailyWinCoins(int nth) {
        return nth < 1 || nth > file.dailyWinCoins.size() ? 0 : scaleCoins(file.dailyWinCoins.get(nth - 1));
    }

    /** Career packs an event pays by match wins (B289); none for events against the AI only */
    public int eventPacks(boolean limited, int wins, boolean withPeople) {
        List<Integer> ladder = limited ? file.eventPrizes.limitedPacks : file.eventPrizes.constructedPacks;
        if (!withPeople || ladder.isEmpty()) {
            return 0;
        }
        return ladder.get(Math.max(0, Math.min(wins, ladder.size() - 1)));
    }

    public int eventXp(int wins) {
        return scaleXp(file.eventPrizes.xp + file.eventPrizes.xpPerWin * Math.max(0, wins));
    }

    /** XP past the top level for each Renown star (B291) */
    public int renownXp() {
        return Math.max(1, file.renownXp);
    }

    /** games a mastery rank takes, rank 1 first (B290) */
    public List<Integer> masteryRankGames() {
        return file.masteryRankGames;
    }

    public int masteryRankXp(int rank) {
        return scaleXp(file.masteryRankXp * rank);
    }

    public int masteryRankCoins(int rank) {
        return scaleCoins(file.masteryRankCoins * rank);
    }

    public List<Integer> dailyWinLadder() {
        List<Integer> ladder = new java.util.ArrayList<>();
        for (int i = 1; i <= file.dailyWinCoins.size(); i++) {
            ladder.add(dailyWinCoins(i));
        }
        return ladder;
    }

    /**
     * XP a game of a kind pays one seat, before the first-win bonuses.
     *
     * @param players seats at the table, for a pod's extra
     * @param aiSkill the strongest AI's skill at an AI table, or 0
     */
    public int gameXp(String kind, boolean won, int players, int aiSkill) {
        Kind rates = file.kinds.get(kind);
        if (rates == null) {
            return 0;
        }
        double xp = won ? rates.winXp : rates.lossXp;
        if (POD.equals(kind) && won) {
            xp += rates.perExtraPlayerXp * Math.max(0, players - 2);
        }
        if (AI.equals(kind) && aiSkill > 0) {
            xp *= file.aiSkill.getOrDefault(String.valueOf(aiSkill), 1.0);
        }
        return scaleXp((int) Math.round(xp));
    }

    static int scaleXp(int xp) {
        return xp * mage.server.admin.RuntimeSettings.get().progressXpPercent() / 100;
    }

    static int scaleCoins(int coins) {
        return coins * mage.server.admin.RuntimeSettings.get().progressCoinPercent() / 100;
    }
}
