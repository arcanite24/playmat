package mage.server.career;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.time.LocalDate;
import java.time.ZoneOffset;
import java.time.temporal.IsoFields;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.function.Supplier;

/**
 * What keeps a Career going between games, as data in resources/career: daily quests (quests.json), the level track
 * (levels.json), achievements (achievements.json), cosmetic unlocks and the weekly win goal. A finished game is applied
 * once, by game id, inside the caller's store transaction.
 */
public final class CareerProgress {

    /** quests that can wait at once */
    public static final int QUEST_SLOTS = 3;
    /** the fourth slot holds the weekly quest */
    public static final int WEEKLY_SLOT = 3;
    /** wins in a week that each give a free pack */
    public static final List<Integer> WEEKLY_GOALS = Collections.unmodifiableList(Arrays.asList(5, 10, 15));
    /** XP each weekly goal pays besides its pack */
    public static final int WEEKLY_GOAL_XP = 250;

    /** counters where a game's value replaces a smaller lifetime value instead of adding to it */
    private static final Set<String> MAX_COUNTERS = new HashSet<>(Arrays.asList(
            "biggest_hit", "max_attacker_power", "most_spells_in_a_turn", "most_mana_in_a_turn", "turns", "life_at_end", "my_turns"));

    // ---- content

    public static class QuestTemplate {
        public String id;
        public String text;
        public String counter;
        public int target;
        public int coins;
        public int xp;
        /** a weekly quest: the fourth slot, a new one each Monday (B288) */
        public boolean weekly;
        /** which games count, in words ("Games against people"); null for any game */
        public String counts;
    }

    public static class AchievementTemplate {
        public String id;
        public String name;
        public String text;
        /** "game": in one game; "total": over the whole Career */
        public String scope;
        public String counter;
        public int target;
        public boolean secret;
        public int coins;
        public int xp;
        public CareerCosmetic cosmetic;
        /** play, social, formats, colors, mechanics, moments, career or long (B305) */
        public String family;
        /** points for earning it; 0 for the default (10, 15 for a secret one) */
        public int points;
        /** bronze, silver, gold and mythic (B304): each its own target and pay; {n} in the text is the target */
        public List<AchievementTier> tiers;

        boolean tiered() {
            return tiers != null && !tiers.isEmpty();
        }

        int pointsOf() {
            return points > 0 ? points : secret ? 15 : 10;
        }
    }

    /** One tier of a tiered achievement. */
    public static class AchievementTier {
        public int target;
        public int coins;
        public int xp;
        public int points;
        public CareerCosmetic cosmetic;
        /** capsule tokens (B314); the tier's place in rewards.json when the file doesn't say */
        public int tokens;
    }

    /** What reaching a level gives. */
    public static class CareerLevelReward {
        public int level;
        public int coins;
        public int packs;
        /** a wildcard of this rarity */
        public String wildcard;
        public CareerCosmetic cosmetic;
        /** capsule tokens (B314): every few levels, by rewards.json */
        public int tokens;
    }

    /** A cosmetic: any kind in the catalog (CareerCosmetics); the catalog says how it looks. */
    public static class CareerCosmetic {
        public String kind;
        public String id;

        public CareerCosmetic() {
        }

        CareerCosmetic(String kind, String id) {
            this.kind = kind;
            this.id = id;
        }
    }

    // ---- views

    /** A quest waiting for the player. */
    public static class CareerQuest {
        public int slot;
        public String id;
        public String text;
        /** which games count, in words; null for any game */
        public String counts;
        public boolean weekly;
        public int progress;
        public int target;
        public int coins;
        public int xp;
    }

    /** A quest in the match menu (B284): its progress before the game and with what this game added so far. */
    public static class CareerLiveQuest {
        public String text;
        public int before;
        public int now;
        public int target;
    }

    /** What anyone sees on a player's profile (B280). */
    public static class CareerPublicProfile {
        public int level;
        public int xp;
        /** XP into the current level, and what the level takes */
        public int levelXp;
        public int levelSize;
        public int wins;
        public int games;
        public int achievements;
        public int achievementsTotal;
        /** achievement points and their rank's name (B307) */
        public int points;
        public String rank;
        public List<CareerStore.PaidGame> recent = new ArrayList<>();
    }

    /** A mastery track (B290): a color or a format, ten ranks from games played. */
    public static class CareerMastery {
        /** "color_W" … "color_colorless", "format_standard" … "format_limited" */
        public String track;
        /** "color" or "format" */
        public String kind;
        public int games;
        public int rank;
        /** games the next rank takes in all; 0 at the top */
        public int nextAt;
        public int maxRank;
    }

    /** A mastery rank reached in a game, and what it paid. */
    public static class CareerMasteryRank {
        public String track;
        public int rank;
        public int xp;
        public int coins;
        public CareerCosmetic cosmetic;
    }

    public static class CareerQuests {
        public List<CareerQuest> quests;
        /** a quest can be swapped for another once a day */
        public boolean canReroll;
        /** a new quest arrives each day (UTC) while fewer than three wait */
        public String today;
    }

    public static class CareerAchievement {
        public String id;
        /** null for a secret achievement not yet earned */
        public String name;
        public String text;
        public boolean secret;
        public boolean achieved;
        public long at;
        public String scope;
        /** the lifetime counter now, for "total" achievements */
        public int progress;
        public int target;
        public int coins;
        public int xp;
        public CareerCosmetic cosmetic;
        /** capsule tokens the tier this game reached gave */
        public int tokens;
        public String family;
        /** tiers earned (0 to 4); for an achievement without tiers, 1 once earned */
        public int tier;
        /** the tiers, bronze first; empty for an achievement without tiers */
        public List<CareerAchievementTier> tiers = new ArrayList<>();
        /** points earned from it so far, and in all */
        public int points;
        public int pointsTotal;
        /** accounts that have it (its first tier), counted on the server; filled by the hall */
        public int holders;
    }

    /** One tier of an achievement, as the hall shows it. */
    public static class CareerAchievementTier {
        public int target;
        public int coins;
        public int xp;
        public int points;
        public CareerCosmetic cosmetic;
        public int tokens;
        /** what this tier asks ("Cast 1,000 spells."); null for a secret achievement's tier not yet earned */
        public String text;
        /** when it was earned; 0 when not yet */
        public long at;
        /** accounts that have this tier */
        public int holders;
    }

    /** The achievement hall (B306): every achievement, the points total with its rank, and what was earned lately. */
    public static class CareerHall {
        public List<CareerAchievement> achievements = new ArrayList<>();
        public int points;
        /** 0-based index into rankNames */
        public int rank;
        public List<String> rankNames = new ArrayList<>();
        /** points each rank starts at */
        public List<Integer> rankPoints = new ArrayList<>();
        /** accounts with progression, for "how many players have it" */
        public int players;
        /** the latest earned (tiers count), newest first */
        public List<CareerAchievement> recent = new ArrayList<>();
    }

    /** A one-game feat told after the game (B309): what, how big and on which turn. */
    public static class CareerMoment {
        public String id;
        public String text;
        public int value;
        public int turn;
    }

    public static class CareerLevelTrack {
        public int level;
        public int xp;
        public int xpIntoLevel;
        /** XP the next level needs; 0 at the top */
        public int xpForNext;
        public List<CareerLevelReward> rewards;
        public List<CareerCosmetic> unlocks;
    }

    public static class CareerWeekly {
        public String week;
        public int wins;
        public List<Integer> goals;
        /** the next goal, or 0 when all are met */
        public int nextGoal;
    }

    /** One quest's change in a game. */
    public static class CareerQuestProgress {
        public String id;
        public String text;
        public int before;
        public int after;
        public int target;
        public boolean completed;
        public int coins;
        public int xp;
    }

    /** What a Career mode (campaign, gauntlet, puzzle, challenge) made of a game, beyond the usual rewards. */
    public static class CareerModeResult {
        /** "campaign", "gauntlet", "puzzle", "challenge" or "limited" */
        public String kind;
        public String title;
        /** what happened, one sentence a line */
        public List<String> lines = new ArrayList<>();
        public int coins;
        public int xp;
        /** a set this game opened in the shop */
        public String unlockedSet;
        /** a puzzle's stars, 1 to 3 */
        public int stars;
        /** the run or week this game belongs to */
        public String ref;
        /** a campaign trial: a set position, not a game */
        public boolean trial;
        /** sleeves, titles and the like this game gave */
        public List<CareerCosmetic> cosmetics = new ArrayList<>();

        public CareerModeResult() {
        }

        CareerModeResult(String kind, String title) {
            this.kind = kind;
            this.title = title;
        }
    }

    /** An extra a game paid: the daily wins ladder, the first win of the day, a match won. */
    public static class CareerBonus {
        public String text;
        public int xp;
        public int coins;
        /** capsule tokens (B314) */
        public int tokens;

        public CareerBonus() {
        }

        CareerBonus(String text, int xp, int coins) {
            this.text = text;
            this.xp = xp;
            this.coins = coins;
        }
    }

    /** Everything a game paid, for the rewards screen after it. */
    public static class CareerGameResult {
        public String gameKey;
        /** the kind of game: "career", "ai", "person", "pod" or "event" */
        public String kind;
        /** the account's XP before this game paid anything, for the XP bar's sweep */
        public int xpBefore;
        /** the day's extras this game paid */
        public List<CareerBonus> bonuses = new ArrayList<>();
        /** whether this was a Career match (else a regular game against the AI that counts for quests) */
        public boolean career;
        public boolean won;
        public String opponent;
        public int coins;
        public int xp;
        public String note;
        public int levelBefore;
        public int levelAfter;
        public int xpIntoLevel;
        public int xpForNext;
        public List<CareerLevelReward> levelRewards = new ArrayList<>();
        public List<CareerQuestProgress> quests = new ArrayList<>();
        /** mastery ranks this game reached (B290) */
        public List<CareerMasteryRank> masteries = new ArrayList<>();
        /** Renown stars past the top level, before and after this game (B291) */
        public int renownBefore;
        public int renownAfter;
        public List<CareerAchievement> achievements = new ArrayList<>();
        /** the game's feats, with their turns (B309) */
        public List<CareerMoment> moments = new ArrayList<>();
        public CareerWeekly weekly;
        /** free packs this game gave */
        public int packs;
        /** capsule tokens this game gave (B314) */
        public int tokens;
        /** opponents' tiers this game opened */
        public List<String> opened = new ArrayList<>();
        /** what a Career mode made of the game; null for a roster duel or a regular game */
        public CareerModeResult mode;
        /** what helped the player in this game (a hint, a take-back), which halves its pay; null when nothing did */
        public String assisted;
    }

    private static volatile CareerProgress instance;

    private final Map<String, QuestTemplate> quests = new LinkedHashMap<>();
    private final Map<String, AchievementTemplate> achievements = new LinkedHashMap<>();
    private final Map<Integer, CareerLevelReward> levels = new LinkedHashMap<>();
    private Supplier<LocalDate> today = () -> LocalDate.now(ZoneOffset.UTC);
    private Random random = new Random();

    private CareerProgress() throws IOException {
        Gson gson = new Gson();
        List<QuestTemplate> questList = read(gson, "quests.json", new TypeToken<List<QuestTemplate>>() {
        });
        questList.forEach(quest -> quests.put(quest.id, quest));
        List<AchievementTemplate> achievementList = read(gson, "achievements.json", new TypeToken<List<AchievementTemplate>>() {
        });
        achievementList.forEach(achievement -> achievements.put(achievement.id, achievement));
        List<CareerLevelReward> levelList = read(gson, "levels.json", new TypeToken<List<CareerLevelReward>>() {
        });
        levelList.forEach(level -> levels.put(level.level, level));
        // capsule tokens (B314): a level every few, and an achievement's higher tiers, unless the file says otherwise
        CareerRewards.Capsules capsules = CareerRewards.get().capsules();
        for (CareerLevelReward level : levels.values()) {
            if (level.tokens == 0 && capsules.levelTokensEvery > 0 && level.level % capsules.levelTokensEvery == 0) {
                level.tokens = capsules.levelTokens;
            }
        }
        for (AchievementTemplate achievement : achievements.values()) {
            if (achievement.tiers != null) {
                for (int tier = 0; tier < achievement.tiers.size() && tier < capsules.tierTokens.size(); tier++) {
                    AchievementTier step = achievement.tiers.get(tier);
                    if (step.tokens == 0) {
                        step.tokens = capsules.tierTokens.get(tier);
                    }
                }
            }
        }
    }

    private static <T> T read(Gson gson, String name, TypeToken<T> type) throws IOException {
        InputStream stream = CareerProgress.class.getResourceAsStream("/career/" + name);
        if (stream == null) {
            throw new IOException("Missing career resource " + name);
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            return gson.fromJson(reader, type.getType());
        }
    }

    /** Today's progress (UTC), for Home's Today panel. */
    public static class CareerDaily {
        public String day;
        public int wins;
        public int games;
        /** the coins each daily win pays, first win first */
        public List<Integer> winCoins = new ArrayList<>();
        /** 1-based step of the ladder the next win pays; past the end when it's done for today */
        public int nextWin;
        public boolean firstWinPaid;
        public boolean firstPersonGamePaid;
        public int firstWinXp;
        public int firstPersonGameXp;
    }

    public CareerDaily daily(String user) throws SQLException {
        CareerRewards rewards = CareerRewards.get();
        CareerDaily view = new CareerDaily();
        view.day = day();
        int[] today = CareerStore.get().daily(user, view.day);
        view.wins = today[0];
        view.games = today[1];
        for (int i = 1; i <= rewards.dailyWinLadder().size(); i++) {
            view.winCoins.add(rewards.dailyWinCoins(i));
        }
        view.nextWin = today[0] + 1;
        view.firstWinPaid = today[0] > 0;
        view.firstPersonGamePaid = today[2] > 0;
        view.firstWinXp = rewards.firstWinXp();
        view.firstPersonGameXp = rewards.firstPersonGameXp();
        return view;
    }

    public static CareerProgress get() {
        CareerProgress current = instance;
        if (current == null) {
            synchronized (CareerProgress.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerProgress();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("Career content can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** for tests: a fixed day and random source */
    void useClock(Supplier<LocalDate> today, Random random) {
        this.today = today;
        this.random = random;
    }

    public Map<String, QuestTemplate> questTemplates() {
        return Collections.unmodifiableMap(quests);
    }

    public Map<String, AchievementTemplate> achievementTemplates() {
        return Collections.unmodifiableMap(achievements);
    }

    public Map<Integer, CareerLevelReward> levelRewards() {
        return Collections.unmodifiableMap(levels);
    }

    String day() {
        return today.get().toString();
    }

    String week() {
        LocalDate date = today.get();
        return date.get(IsoFields.WEEK_BASED_YEAR) + "-W" + String.format("%02d", date.get(IsoFields.WEEK_OF_WEEK_BASED_YEAR));
    }

    // ---- quests

    /**
     * Hands out the day's quest: a new Career gets three, then one arrives each day while fewer than three wait.
     * Runs inside the caller's transaction.
     */
    void refreshQuests(CareerStore store, String user) throws SQLException {
        refreshWeeklyQuest(store, user);
        String day = day();
        String last = store.profileText(user, "quest_day");
        if (day.equals(last)) {
            return;
        }
        List<CareerStore.QuestRow> waiting = new ArrayList<>(store.quests(user));
        waiting.removeIf(row -> row.slot == WEEKLY_SLOT);
        int toAdd = last == null ? QUEST_SLOTS - waiting.size() : Math.min(1, QUEST_SLOTS - waiting.size());
        Set<Integer> used = new HashSet<>();
        Set<String> present = new HashSet<>();
        for (CareerStore.QuestRow row : waiting) {
            used.add(row.slot);
            present.add(row.quest);
        }
        for (int i = 0; i < toAdd; i++) {
            int slot = 0;
            while (used.contains(slot)) {
                slot++;
            }
            String quest = pickQuest(present, false);
            if (quest == null) {
                break;
            }
            store.putQuest(user, slot, quest, 0, day);
            used.add(slot);
            present.add(quest);
        }
        store.setProfileText(user, "quest_day", day);
    }

    /** the weekly slot: a new weekly quest each Monday, whatever happened to last week's */
    private void refreshWeeklyQuest(CareerStore store, String user) throws SQLException {
        String week = week();
        if (week.equals(store.profileText(user, "quest_week"))) {
            return;
        }
        Set<String> present = new HashSet<>();
        for (CareerStore.QuestRow row : store.quests(user)) {
            present.add(row.quest);
        }
        String quest = pickQuest(present, true);
        if (quest != null) {
            store.putQuest(user, WEEKLY_SLOT, quest, 0, day());
        }
        store.setProfileText(user, "quest_week", week);
    }

    private String pickQuest(Set<String> exclude, boolean weekly) {
        List<String> choices = new ArrayList<>();
        for (String id : quests.keySet()) {
            if (!exclude.contains(id) && quests.get(id).weekly == weekly) {
                choices.add(id);
            }
        }
        return choices.isEmpty() ? null : choices.get(random.nextInt(choices.size()));
    }

    public CareerQuests quests(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        return store.transaction(() -> {
            if (store.profile(user) == null) {
                return null;
            }
            refreshQuests(store, user);
            return questsView(store, user);
        });
    }

    private CareerQuests questsView(CareerStore store, String user) throws SQLException {
        CareerQuests view = new CareerQuests();
        view.quests = new ArrayList<>();
        for (CareerStore.QuestRow row : store.quests(user)) {
            QuestTemplate template = quests.get(row.quest);
            if (template == null) {
                continue; // a quest removed from the catalogue waits silently until rerolled
            }
            CareerQuest quest = new CareerQuest();
            quest.slot = row.slot;
            quest.id = template.id;
            quest.text = template.text;
            quest.counts = template.counts;
            quest.weekly = template.weekly;
            quest.progress = row.progress;
            quest.target = template.target;
            quest.coins = template.coins;
            quest.xp = template.xp;
            view.quests.add(quest);
        }
        view.canReroll = !day().equals(store.profileText(user, "reroll_day"));
        view.today = day();
        return view;
    }

    /** the player's quests with what the game so far adds, from the game's live tally; nothing is saved */
    public List<CareerLiveQuest> liveQuests(String user, Map<String, Integer> counters) throws SQLException {
        CareerStore store = CareerStore.get();
        List<CareerLiveQuest> live = new ArrayList<>();
        if (store.profile(user) == null) {
            return live;
        }
        for (CareerStore.QuestRow row : store.quests(user)) {
            QuestTemplate template = quests.get(row.quest);
            if (template == null || row.progress >= template.target) {
                continue;
            }
            CareerLiveQuest quest = new CareerLiveQuest();
            quest.text = template.text;
            quest.before = row.progress;
            quest.now = Math.min(template.target, row.progress + counters.getOrDefault(template.counter, 0));
            quest.target = template.target;
            live.add(quest);
        }
        return live;
    }

    public CareerPublicProfile publicProfile(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null) {
            return null;
        }
        CareerPublicProfile view = new CareerPublicProfile();
        view.level = profile.level;
        view.xp = profile.xp;
        int left = profile.xp;
        for (int level = 1; level < profile.level; level++) {
            left -= CareerRules.xpForLevel(level);
        }
        view.levelXp = left;
        view.levelSize = CareerRules.xpForLevel(profile.level);
        Map<String, Long> earned = store.achievements(user);
        view.achievements = earned.size();
        for (AchievementTemplate template : achievements.values()) {
            view.achievementsTotal += template.tiered() ? template.tiers.size() : 1;
            view.points += pointsEarned(template, earned);
        }
        view.rank = POINT_RANKS.get(pointRank(view.points));
        for (CareerStore.PaidGame game : store.paidGames(user, 200)) {
            view.games++;
            if (game.won) {
                view.wins++;
            }
            if (view.recent.size() < 10) {
                view.recent.add(game);
            }
        }
        return view;
    }

    /** swaps the quest in a slot for another one; once a day */
    public CareerQuests reroll(String user, int slot) throws SQLException, CareerService.CareerException {
        CareerStore store = CareerStore.get();
        try {
            return store.transaction(() -> {
                if (store.profile(user) == null) {
                    throw new SQLException(new CareerService.CareerException("Open a Career first."));
                }
                if (day().equals(store.profileText(user, "reroll_day"))) {
                    throw new SQLException(new CareerService.CareerException("You've already swapped a quest today."));
                }
                CareerStore.QuestRow row = store.quests(user).stream().filter(quest -> quest.slot == slot).findFirst().orElse(null);
                if (row == null) {
                    throw new SQLException(new CareerService.CareerException("There's no quest in that slot."));
                }
                Set<String> present = new HashSet<>();
                store.quests(user).forEach(quest -> present.add(quest.quest));
                if (slot == WEEKLY_SLOT) {
                    throw new SQLException(new CareerService.CareerException("The weekly quest can't be swapped."));
                }
                String next = pickQuest(present, false);
                if (next == null) {
                    throw new SQLException(new CareerService.CareerException("There's no other quest to swap in."));
                }
                store.putQuest(user, slot, next, 0, day());
                store.setProfileText(user, "reroll_day", day());
                return questsView(store, user);
            });
        } catch (SQLException e) {
            if (e.getCause() instanceof CareerService.CareerException) {
                throw (CareerService.CareerException) e.getCause();
            }
            throw e;
        }
    }

    // ---- applying a game

    /**
     * Applies a finished game's tally: lifetime stats, quests, the weekly goal (Career wins only), achievements and
     * level rewards. Runs inside the caller's transaction; the caller has already claimed the game and added the
     * match's own coins and XP.
     *
     * @param result filled in; its coins and xp already hold the match's own pay
     * @param xpBefore the account's XP before this game paid anything
     */
    void apply(CareerStore store, String user, CareerTally tally, CareerGameResult result, int xpBefore, long now) throws SQLException {
        refreshQuests(store, user);
        Map<String, Integer> counters = new LinkedHashMap<>(tally.counters());
        // days played, never streaks (B305): the first counted game of a UTC day adds one
        int today = Integer.parseInt(day().replace("-", ""));
        if (store.stats(user).getOrDefault("last_day", 0) < today) {
            store.maxStat(user, "last_day", today);
            counters.merge("days_played", 1, Integer::sum);
        }
        for (CareerTally.Moment moment : tally.moments()) {
            result.moments.add(moment(moment));
        }

        // lifetime stats
        for (Map.Entry<String, Integer> entry : counters.entrySet()) {
            if (MAX_COUNTERS.contains(entry.getKey())) {
                store.maxStat(user, entry.getKey(), entry.getValue());
            } else {
                store.addStat(user, entry.getKey(), entry.getValue());
            }
        }

        // quests
        for (CareerStore.QuestRow row : store.quests(user)) {
            QuestTemplate template = quests.get(row.quest);
            int gained = template == null ? 0 : counters.getOrDefault(template.counter, 0);
            if (gained <= 0) {
                continue;
            }
            CareerQuestProgress progress = new CareerQuestProgress();
            progress.id = template.id;
            progress.text = template.text;
            progress.before = row.progress;
            progress.after = Math.min(template.target, row.progress + gained);
            progress.target = template.target;
            if (progress.after >= template.target) {
                progress.completed = true;
                progress.coins = template.coins;
                progress.xp = template.xp;
                store.deleteQuest(user, row.slot);
                store.addCoins(user, template.coins);
                store.addProfileInt(user, "xp", template.xp);
                store.addStat(user, "quests_completed", 1);
                result.coins += template.coins;
                result.xp += template.xp;
            } else {
                store.putQuest(user, row.slot, row.quest, progress.after, row.assigned);
            }
            result.quests.add(progress);
        }

        // the weekly goal: every win of any kind (B287), but not puzzles, trials or the weekly challenge
        boolean weeklyGame = result.mode == null
                || !("puzzle".equals(result.mode.kind) || "challenge".equals(result.mode.kind) || result.mode.trial);
        if (result.won && weeklyGame) {
            String week = week();
            int before = store.weeklyWins(user, week);
            store.addWeeklyWin(user, week);
            int after = before + 1;
            if (WEEKLY_GOALS.contains(after)) {
                store.addProfileInt(user, "pack_tokens", 1);
                result.packs++;
                int xp = CareerRewards.scaleXp(WEEKLY_GOAL_XP);
                store.addProfileInt(user, "xp", xp);
                result.xp += xp;
                CareerBonus bonus = new CareerBonus("Weekly goal: " + after + " wins", xp, 0);
                bonus.tokens = CareerRewards.get().capsules().weeklyGoalTokens;
                if (bonus.tokens > 0) {
                    CareerCapsules.addTokens(store, user, bonus.tokens, "weekly goal", System.currentTimeMillis());
                    result.tokens += bonus.tokens;
                }
                result.bonuses.add(bonus);
            }
            store.maxStat(user, "weekly_best", after);
        }
        result.weekly = weekly(store, user);

        masteriesIn(store, user, result, now);
        achievementsIn(store, user, counters, result, now);
        levelsIn(store, user, xpBefore, result, now);
    }

    /** the color and format tracks, in the order the client lists them */
    public static List<String> masteryTracks() {
        List<String> tracks = new ArrayList<>();
        for (String color : Arrays.asList("W", "U", "B", "R", "G", "multicolor", "colorless")) {
            tracks.add("color_" + color);
        }
        for (String format : CareerService.FORMATS) {
            tracks.add("format_" + format);
        }
        return tracks;
    }

    static int masteryRank(int games) {
        List<Integer> ranks = CareerRewards.get().masteryRankGames();
        int rank = 0;
        while (rank < ranks.size() && games >= ranks.get(rank)) {
            rank++;
        }
        return rank;
    }

    /** every mastery track with its games and rank */
    public List<CareerMastery> masteries(String user) throws SQLException {
        Map<String, Integer> stats = CareerStore.get().stats(user);
        List<Integer> ranks = CareerRewards.get().masteryRankGames();
        List<CareerMastery> view = new ArrayList<>();
        for (String track : masteryTracks()) {
            CareerMastery mastery = new CareerMastery();
            mastery.track = track;
            mastery.kind = track.startsWith("color_") ? "color" : "format";
            mastery.games = stats.getOrDefault("games_" + track, 0);
            mastery.rank = masteryRank(mastery.games);
            mastery.maxRank = ranks.size();
            mastery.nextAt = mastery.rank < ranks.size() ? ranks.get(mastery.rank) : 0;
            view.add(mastery);
        }
        return view;
    }

    /** pays every mastery rank reached since the last one paid: XP, coins, a badge, and a title at the top */
    void masteriesIn(CareerStore store, String user, CareerGameResult result, long now) throws SQLException {
        CareerRewards rewards = CareerRewards.get();
        int top = rewards.masteryRankGames().size();
        Map<String, Integer> stats = store.stats(user);
        for (String track : masteryTracks()) {
            int rank = masteryRank(stats.getOrDefault("games_" + track, 0));
            int paid = stats.getOrDefault("mastery_" + track, 0);
            for (int next = paid + 1; next <= rank; next++) {
                CareerMasteryRank reached = new CareerMasteryRank();
                reached.track = track;
                reached.rank = next;
                reached.xp = rewards.masteryRankXp(next);
                reached.coins = rewards.masteryRankCoins(next);
                store.addProfileInt(user, "xp", reached.xp);
                store.addCoins(user, reached.coins);
                store.addUnlock(user, "badge", ("mastery-" + track.replace('_', '-') + "-" + next).toLowerCase(java.util.Locale.ROOT), now);
                reached.cosmetic = masteryCosmetic(track, next, top);
                if (reached.cosmetic != null) {
                    store.addUnlock(user, reached.cosmetic.kind, reached.cosmetic.id, now);
                }
                result.xp += reached.xp;
                result.coins += reached.coins;
                result.masteries.add(reached);
            }
            if (rank > paid) {
                store.maxStat(user, "mastery_" + track, rank);
                store.addStat(user, "mastery_ranks", rank - paid);
            }
        }
    }

    /** the cosmetic a mastery rank gives: the catalog's, or at the top the track's master title */
    private static CareerCosmetic masteryCosmetic(String track, int rank, int top) {
        CareerCosmetic cosmetic = null;
        try {
            cosmetic = CareerCosmetics.get().masteryReward(track, rank);
        } catch (RuntimeException e) {
            // no catalog
        }
        if (cosmetic == null && rank == top) {
            cosmetic = new CareerCosmetic("title", track.substring(track.indexOf('_') + 1).toLowerCase(java.util.Locale.ROOT) + "-master");
        }
        return cosmetic;
    }

    /** the store's id for an achievement's tier (1-based); an achievement without tiers is its own id */
    static String tierKey(String id, int tier) {
        return id + ":" + tier;
    }

    /** the badge an earned achievement (or one tier of it) shows: its id, or id-tier */
    static String badgeOf(String storedId) {
        return storedId.replace(':', '-');
    }

    /** checks every achievement (and tier) not yet earned; game ones against this game's counters (if any) */
    void achievementsIn(CareerStore store, String user, Map<String, Integer> game, CareerGameResult result, long now) throws SQLException {
        Map<String, Long> earned = store.achievements(user);
        Map<String, Integer> totals = totals(store, user);
        for (AchievementTemplate template : achievements.values()) {
            int value = "game".equals(template.scope)
                    ? (game == null ? 0 : game.getOrDefault(template.counter, 0))
                    : totals.getOrDefault(template.counter, 0);
            if (!template.tiered()) {
                if (earned.containsKey(template.id) || value < template.target || !store.addAchievement(user, template.id, now)) {
                    continue;
                }
                pay(store, user, template.coins, template.xp, template.cosmetic, now);
                if (result != null) {
                    result.coins += template.coins;
                    result.xp += template.xp;
                    result.achievements.add(view(template, earned(earned, template.id, now), value));
                }
                continue;
            }
            for (int tier = 1; tier <= template.tiers.size(); tier++) {
                AchievementTier step = template.tiers.get(tier - 1);
                String key = tierKey(template.id, tier);
                if (earned.containsKey(key)) {
                    continue;
                }
                if (value < step.target || !store.addAchievement(user, key, now)) {
                    break;
                }
                earned.put(key, now);
                pay(store, user, step.coins, step.xp, step.cosmetic, now);
                if (step.tokens > 0) {
                    CareerCapsules.addTokens(store, user, step.tokens, "achievement " + key, now);
                }
                if (result != null) {
                    result.coins += step.coins;
                    result.xp += step.xp;
                    result.tokens += step.tokens;
                    CareerAchievement view = view(template, earned, value);
                    // the tier this game reached, as the rewards show it
                    view.tier = tier;
                    view.target = step.target;
                    view.text = text(template, step.target);
                    view.coins = step.coins;
                    view.xp = step.xp;
                    view.cosmetic = step.cosmetic;
                    view.tokens = step.tokens;
                    result.achievements.add(view);
                }
            }
        }
    }

    private static Map<String, Long> earned(Map<String, Long> earned, String id, long at) {
        Map<String, Long> copy = new LinkedHashMap<>(earned);
        copy.put(id, at);
        return copy;
    }

    private static void pay(CareerStore store, String user, int coins, int xp, CareerCosmetic cosmetic, long now) throws SQLException {
        if (coins > 0) {
            store.addCoins(user, coins);
        }
        if (xp > 0) {
            store.addProfileInt(user, "xp", xp);
        }
        if (cosmetic != null) {
            store.addUnlock(user, cosmetic.kind, cosmetic.id, now);
        }
    }

    /** an achievement's text for one target: "Cast {n} spells." reads "Cast 1,000 spells." */
    static String text(AchievementTemplate template, int target) {
        return template.text == null ? null : template.text.replace("{n}", String.format(java.util.Locale.ROOT, "%,d", target));
    }

    private static CareerMoment moment(CareerTally.Moment moment) {
        CareerMoment view = new CareerMoment();
        view.id = moment.id;
        view.value = moment.value;
        view.turn = moment.turn;
        switch (moment.id) {
            case "big_hit":
                view.text = "Hit for " + moment.value + " in one blow";
                break;
            case "storm":
                view.text = "Cast " + moment.value + " spells in one turn";
                break;
            case "mana_surge":
                view.text = "Made " + moment.value + " mana in one turn";
                break;
            case "life_swing":
                view.text = "Gained " + moment.value + " life at once";
                break;
            case "elimination":
                view.text = moment.value == 1 ? "Knocked a player out" : "Knocked out " + moment.value + " players";
                break;
            case "commander_hit":
                view.text = "Your commander hit for " + moment.value;
                break;
            case "one_life":
                view.text = "Won at 1 life";
                break;
            case "comeback":
                view.text = "Came back from " + moment.value + " life";
                break;
            case "flawless":
                view.text = "Won without taking damage";
                break;
            case "fast_win":
                view.text = "Won on your turn " + moment.value;
                break;
            default:
                view.text = moment.id;
        }
        return view;
    }

    /** points: the named ranks and where each starts (B307) */
    public static final List<String> POINT_RANKS = Arrays.asList("Novice", "Apprentice", "Adept", "Expert", "Master", "Grandmaster", "Legend");
    public static final List<Integer> POINT_RANK_STARTS = Arrays.asList(0, 100, 300, 750, 1500, 3000, 5000);

    static int pointRank(int points) {
        int rank = 0;
        while (rank + 1 < POINT_RANK_STARTS.size() && points >= POINT_RANK_STARTS.get(rank + 1)) {
            rank++;
        }
        return rank;
    }

    /** the account's achievement points */
    public int points(String user) throws SQLException {
        Map<String, Long> earned = CareerStore.get().achievements(user);
        int points = 0;
        for (AchievementTemplate template : achievements.values()) {
            points += pointsEarned(template, earned);
        }
        return points;
    }

    private static int pointsEarned(AchievementTemplate template, Map<String, Long> earned) {
        if (!template.tiered()) {
            return earned.containsKey(template.id) ? template.pointsOf() : 0;
        }
        int points = 0;
        for (int tier = 1; tier <= template.tiers.size(); tier++) {
            if (earned.containsKey(tierKey(template.id, tier))) {
                points += template.tiers.get(tier - 1).points;
            }
        }
        return points;
    }

    /** the badges an account may show (B307): every achievement and tier earned, and its mastery ranks */
    public java.util.Set<String> badges(String user) throws SQLException {
        java.util.Set<String> badges = new java.util.HashSet<>();
        CareerStore store = CareerStore.get();
        for (String id : store.achievements(user).keySet()) {
            badges.add(badgeOf(id).toLowerCase(java.util.Locale.ROOT));
        }
        for (String[] unlock : store.unlocks(user)) {
            if ("badge".equals(unlock[0])) {
                badges.add(unlock[1].toLowerCase(java.util.Locale.ROOT));
            }
        }
        return badges;
    }

    /** the achievement hall: every achievement with tier holders, points and rank, the latest earned */
    public CareerHall hall(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerHall hall = new CareerHall();
        hall.achievements = achievements(user);
        Map<String, Integer> counts = store.achievementCounts();
        Map<String, Long> earned = store.achievements(user);
        List<CareerAchievement> recent = new ArrayList<>();
        for (CareerAchievement view : hall.achievements) {
            AchievementTemplate template = achievements.get(view.id);
            if (template == null) {
                continue;
            }
            if (template.tiered()) {
                for (int tier = 1; tier <= view.tiers.size(); tier++) {
                    view.tiers.get(tier - 1).holders = counts.getOrDefault(tierKey(view.id, tier), 0);
                    Long at = earned.get(tierKey(view.id, tier));
                    if (at != null) {
                        CareerAchievement one = view(template, earned, view.progress);
                        one.tier = tier;
                        one.at = at;
                        one.target = template.tiers.get(tier - 1).target;
                        one.text = text(template, one.target);
                        recent.add(one);
                    }
                }
                view.holders = view.tiers.isEmpty() ? 0 : view.tiers.get(0).holders;
            } else {
                view.holders = counts.getOrDefault(view.id, 0);
                if (view.achieved) {
                    recent.add(view);
                }
            }
            hall.points += view.points;
        }
        recent.sort((a, b) -> Long.compare(b.at, a.at));
        hall.recent = new ArrayList<>(recent.subList(0, Math.min(8, recent.size())));
        hall.rank = pointRank(hall.points);
        hall.rankNames = POINT_RANKS;
        hall.rankPoints = POINT_RANK_STARTS;
        hall.players = store.profileCount();
        return hall;
    }

    /** lifetime counters, plus the ones read from the rest of the Career */
    private Map<String, Integer> totals(CareerStore store, String user) throws SQLException {
        Map<String, Integer> totals = new LinkedHashMap<>(store.stats(user));
        CareerProfile profile = store.profile(user);
        totals.put("level", profile == null ? 1 : profile.level);
        Map<String, Integer> wins = store.winsByOpponent(user);
        int beaten = 0;
        try {
            for (CareerContent.Opponent opponent : CareerContent.get().opponents()) {
                if (wins.getOrDefault(opponent.id, 0) > 0) {
                    beaten++;
                }
            }
        } catch (RuntimeException e) {
            // no roster content
        }
        totals.put("opponents_beaten", beaten);
        int tiersOpen = 1;
        try {
            for (CareerService.CareerOpponent opponent : CareerService.get().opponents(user)) {
                if (opponent.unlocked) {
                    tiersOpen = Math.max(tiersOpen, opponent.tier);
                }
            }
        } catch (RuntimeException e) {
            // no roster (tests without content): only the first tier counts
        }
        totals.put("tiers_open", tiersOpen);
        return totals;
    }

    /** pays every level reached since the last one paid */
    void levelsIn(CareerStore store, String user, int xpBefore, CareerGameResult result, long now) throws SQLException {
        CareerProfile profile = store.profile(user);
        result.levelBefore = CareerRules.levelFor(xpBefore);
        result.levelAfter = profile.level;
        int paid = Math.max(1, store.profileInt(user, "level_paid"));
        for (int level = paid + 1; level <= profile.level; level++) {
            CareerLevelReward reward = levels.get(level);
            if (reward != null) {
                grant(store, user, reward, now);
                result.levelRewards.add(reward);
                result.coins += reward.coins;
                result.packs += reward.packs;
                result.tokens += reward.tokens;
            }
        }
        if (profile.level > paid) {
            store.setProfileInt(user, "level_paid", profile.level);
        }
        // Renown past the top level: a pack for each star (B291)
        int renownXp = CareerRewards.get().renownXp();
        result.renownBefore = CareerRules.renownFor(xpBefore, renownXp);
        result.renownAfter = CareerRules.renownFor(profile.xp, renownXp);
        int renownPaid = store.stats(user).getOrDefault("renown", 0);
        if (result.renownAfter > renownPaid) {
            store.addProfileInt(user, "pack_tokens", result.renownAfter - renownPaid);
            result.packs += result.renownAfter - renownPaid;
            store.maxStat(user, "renown", result.renownAfter);
        }
        int[] into = xpIntoLevel(profile.xp);
        result.xpIntoLevel = into[0];
        result.xpForNext = into[1];
    }

    private static void grant(CareerStore store, String user, CareerLevelReward reward, long now) throws SQLException {
        if (reward.coins > 0) {
            store.addCoins(user, reward.coins);
        }
        if (reward.packs > 0) {
            store.addProfileInt(user, "pack_tokens", reward.packs);
        }
        if (reward.wildcard != null) {
            store.addWildcard(user, reward.wildcard, 1);
        }
        if (reward.cosmetic != null) {
            store.addUnlock(user, reward.cosmetic.kind, reward.cosmetic.id, now);
        }
        if (reward.tokens > 0) {
            CareerCapsules.addTokens(store, user, reward.tokens, "level " + reward.level, now);
        }
    }

    /** XP into the current level and XP the next one needs (0 at the top) */
    static int[] xpIntoLevel(int xp) {
        int level = 1;
        int left = xp;
        while (level < CareerRules.MAX_LEVEL && left >= CareerRules.xpForLevel(level)) {
            left -= CareerRules.xpForLevel(level);
            level++;
        }
        return level >= CareerRules.MAX_LEVEL ? new int[]{left, 0} : new int[]{left, CareerRules.xpForLevel(level)};
    }

    // ---- views for the client

    public CareerWeekly weekly(CareerStore store, String user) throws SQLException {
        CareerWeekly weekly = new CareerWeekly();
        weekly.week = week();
        weekly.wins = store.weeklyWins(user, weekly.week);
        weekly.goals = WEEKLY_GOALS;
        weekly.nextGoal = WEEKLY_GOALS.stream().filter(goal -> goal > weekly.wins).findFirst().orElse(0);
        return weekly;
    }

    public List<CareerAchievement> achievements(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        Map<String, Long> earned = store.achievements(user);
        Map<String, Integer> totals = totals(store, user);
        List<CareerAchievement> views = new ArrayList<>();
        for (AchievementTemplate template : achievements.values()) {
            int progress = "total".equals(template.scope) ? totals.getOrDefault(template.counter, 0) : 0;
            views.add(view(template, earned, progress));
        }
        return views;
    }

    /**
     * An achievement as the player sees it. With tiers, the target, pay and text are the next tier's (the top one's
     * once all are earned), and progress counts toward it; achieved means the first tier is.
     */
    private static CareerAchievement view(AchievementTemplate template, Map<String, Long> earned, int value) {
        CareerAchievement view = new CareerAchievement();
        view.id = template.id;
        view.secret = template.secret;
        view.scope = template.scope;
        view.family = template.family;
        if (!template.tiered()) {
            Long at = earned.get(template.id);
            view.achieved = at != null;
            view.at = at == null ? 0 : at;
            view.tier = view.achieved ? 1 : 0;
            view.target = template.target;
            view.progress = view.achieved ? template.target : Math.min(template.target, value);
            view.coins = template.coins;
            view.xp = template.xp;
            view.cosmetic = template.cosmetic;
            view.points = view.achieved ? template.pointsOf() : 0;
            view.pointsTotal = template.pointsOf();
            if (view.achieved || !template.secret) {
                view.name = template.name;
                view.text = text(template, template.target);
            }
            return view;
        }
        AchievementTier next = null;
        for (int tier = 1; tier <= template.tiers.size(); tier++) {
            AchievementTier step = template.tiers.get(tier - 1);
            Long at = earned.get(tierKey(template.id, tier));
            CareerAchievementTier tierView = new CareerAchievementTier();
            tierView.target = step.target;
            tierView.coins = step.coins;
            tierView.xp = step.xp;
            tierView.points = step.points;
            tierView.cosmetic = step.cosmetic;
            tierView.tokens = step.tokens;
            tierView.at = at == null ? 0 : at;
            if (at != null || !template.secret) {
                tierView.text = text(template, step.target);
            }
            view.tiers.add(tierView);
            view.pointsTotal += step.points;
            if (at != null) {
                view.tier = tier;
                view.points += step.points;
                view.at = Math.max(view.at, at);
            } else if (next == null) {
                next = step;
            }
        }
        AchievementTier shown = next != null ? next : template.tiers.get(template.tiers.size() - 1);
        view.achieved = view.tier > 0;
        view.target = shown.target;
        view.progress = Math.min(shown.target, value);
        view.coins = shown.coins;
        view.xp = shown.xp;
        view.cosmetic = shown.cosmetic;
        if (view.achieved || !template.secret) {
            view.name = template.name;
            view.text = text(template, shown.target);
        }
        return view;
    }

    public CareerLevelTrack levelTrack(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null) {
            return null;
        }
        CareerLevelTrack track = new CareerLevelTrack();
        track.level = profile.level;
        track.xp = profile.xp;
        int[] into = xpIntoLevel(profile.xp);
        track.xpIntoLevel = into[0];
        track.xpForNext = into[1];
        track.rewards = new ArrayList<>(levels.values());
        track.unlocks = new ArrayList<>();
        for (String[] unlock : store.unlocks(user)) {
            track.unlocks.add(new CareerCosmetic(unlock[0], unlock[1]));
        }
        return track;
    }
}
