package mage.server.career;

/**
 * The Career's numbers in one place: what games pay, what packs and crafting cost, levels. Tuned with the economy
 * model (Mage.Server/src/test/java/mage/server/career/CareerEconomyTest.java): a new pack every three or four wins.
 */
public final class CareerRules {

    public static final String COMMON = "common";
    public static final String UNCOMMON = "uncommon";
    public static final String RARE = "rare";
    public static final String MYTHIC = "mythic";
    /** basic lands: free and unlimited, never counted */
    public static final String LAND = "land";

    /** coins a new Career starts with: one pack */
    public static final int START_COINS = 100;
    public static final int PACK_PRICE = 100;
    /** copies of a card that count; more become coins or wildcards */
    public static final int PLAYSET = 4;
    /** a loss that ended before this turn pays nothing (no conceding for coins) */
    public static final int MIN_TURN_FOR_REWARD = 3;
    /** the top of the level track; past it, Renown (B291) */
    public static final int MAX_LEVEL = 100;
    /** wins against a tier's opponents that open the next tier */
    public static final int WINS_TO_UNLOCK_NEXT_TIER = 3;

    private CareerRules() {
    }

    public static class Reward {
        public final int coins;
        public final int xp;
        public final String note;

        public Reward(int coins, int xp, String note) {
            this.coins = coins;
            this.xp = xp;
            this.note = note;
        }
    }

    /**
     * What a Career match pays.
     *
     * @param tier 1 to 5, the opponent's tier
     * @param turns the turn the last game ended on
     */
    public static Reward reward(int tier, boolean won, int turns) {
        if (won) {
            return new Reward(winCoins(tier), 100 + 25 * (tier - 1), null);
        }
        if (turns < MIN_TURN_FOR_REWARD) {
            return new Reward(0, 0, "The game ended before turn " + MIN_TURN_FOR_REWARD + ", so it paid nothing.");
        }
        return new Reward(5, 25, null);
    }

    public static int winCoins(int tier) {
        switch (tier) {
            case 1:
                return 25;
            case 2:
                return 30;
            case 3:
                return 35;
            case 4:
                return 45;
            default:
                return 60;
        }
    }

    /** coins a copy past the playset is worth; rares and mythics become wildcards instead (0 here) */
    public static int spareCoins(String rarity) {
        switch (rarity) {
            case COMMON:
                return 5;
            case UNCOMMON:
                return 10;
            default:
                return 0;
        }
    }

    /** coins to craft a common or uncommon; rares and mythics take a wildcard (0 here) */
    public static int craftCoins(String rarity) {
        switch (rarity) {
            case COMMON:
                return 25;
            case UNCOMMON:
                return 50;
            default:
                return 0;
        }
    }

    /** XP from one level to the next: 250, then 50 more for each level */
    public static int xpForLevel(int level) {
        return 250 + 50 * (level - 1);
    }

    /** XP from zero to the start of a level */
    public static int xpToReach(int level) {
        int total = 0;
        for (int past = 1; past < Math.min(level, MAX_LEVEL); past++) {
            total += xpForLevel(past);
        }
        return total;
    }

    /** Renown stars past the top level: one for every renownXp XP beyond it */
    public static int renownFor(int xp, int renownXp) {
        int past = xp - xpToReach(MAX_LEVEL);
        return past <= 0 ? 0 : past / renownXp;
    }

    public static int levelFor(int xp) {
        int level = 1;
        int left = xp;
        while (level < MAX_LEVEL && left >= xpForLevel(level)) {
            left -= xpForLevel(level);
            level++;
        }
        return level;
    }

    /** the rarity name Career uses for an engine rarity */
    public static String rarityOf(mage.constants.Rarity rarity) {
        if (rarity == null) {
            return COMMON;
        }
        switch (rarity) {
            case LAND:
                return LAND;
            case UNCOMMON:
                return UNCOMMON;
            case RARE:
            case SPECIAL:
            case BONUS:
                return RARE;
            case MYTHIC:
                return MYTHIC;
            default:
                return COMMON;
        }
    }
}
