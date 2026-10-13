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
    /** wins in a week that each give a free pack */
    public static final List<Integer> WEEKLY_GOALS = Collections.unmodifiableList(Arrays.asList(5, 10, 15));

    /** counters where a game's value replaces a smaller lifetime value instead of adding to it */
    private static final Set<String> MAX_COUNTERS = new HashSet<>(Arrays.asList(
            "biggest_hit", "max_attacker_power", "most_spells_in_a_turn", "turns", "life_at_end", "my_turns"));

    // ---- content

    public static class QuestTemplate {
        public String id;
        public String text;
        public String counter;
        public int target;
        public int coins;
        public int xp;
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
    }

    /** What reaching a level gives. */
    public static class CareerLevelReward {
        public int level;
        public int coins;
        public int packs;
        /** a wildcard of this rarity */
        public String wildcard;
        public CareerCosmetic cosmetic;
    }

    /** A cosmetic: a sleeve, playmat, avatar or title. The client knows how each looks. */
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
        public int progress;
        public int target;
        public int coins;
        public int xp;
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

    /** Everything a game paid, for the rewards screen after it. */
    public static class CareerGameResult {
        public String gameKey;
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
        public List<CareerAchievement> achievements = new ArrayList<>();
        public CareerWeekly weekly;
        /** free packs this game gave */
        public int packs;
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
        String day = day();
        String last = store.profileText(user, "quest_day");
        if (day.equals(last)) {
            return;
        }
        List<CareerStore.QuestRow> waiting = store.quests(user);
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
            String quest = pickQuest(present);
            if (quest == null) {
                break;
            }
            store.putQuest(user, slot, quest, 0, day);
            used.add(slot);
            present.add(quest);
        }
        store.setProfileText(user, "quest_day", day);
    }

    private String pickQuest(Set<String> exclude) {
        List<String> choices = new ArrayList<>();
        for (String id : quests.keySet()) {
            if (!exclude.contains(id)) {
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
                String next = pickQuest(present);
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
        Map<String, Integer> counters = tally.counters();

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

        // the weekly goal: Career wins only, and not puzzles or the weekly challenge
        boolean weeklyGame = result.mode == null
                || !("puzzle".equals(result.mode.kind) || "challenge".equals(result.mode.kind) || result.mode.trial);
        if (result.career && result.won && weeklyGame) {
            String week = week();
            int before = store.weeklyWins(user, week);
            store.addWeeklyWin(user, week);
            int after = before + 1;
            if (WEEKLY_GOALS.contains(after)) {
                store.addProfileInt(user, "pack_tokens", 1);
                result.packs++;
            }
            store.maxStat(user, "weekly_best", after);
        }
        result.weekly = weekly(store, user);

        achievementsIn(store, user, counters, result, now);
        levelsIn(store, user, xpBefore, result, now);
    }

    /** checks every achievement not yet earned; game ones against this game's counters (if any) */
    void achievementsIn(CareerStore store, String user, Map<String, Integer> game, CareerGameResult result, long now) throws SQLException {
        Map<String, Long> earned = store.achievements(user);
        Map<String, Integer> totals = totals(store, user);
        for (AchievementTemplate template : achievements.values()) {
            if (earned.containsKey(template.id)) {
                continue;
            }
            int value = "game".equals(template.scope)
                    ? (game == null ? 0 : game.getOrDefault(template.counter, 0))
                    : totals.getOrDefault(template.counter, 0);
            if (value < template.target || !store.addAchievement(user, template.id, now)) {
                continue;
            }
            if (template.coins > 0) {
                store.addCoins(user, template.coins);
            }
            if (template.xp > 0) {
                store.addProfileInt(user, "xp", template.xp);
            }
            if (template.cosmetic != null) {
                store.addUnlock(user, template.cosmetic.kind, template.cosmetic.id, now);
            }
            if (result != null) {
                result.coins += template.coins;
                result.xp += template.xp;
                result.achievements.add(view(template, true, now, value));
            }
        }
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
            }
        }
        if (profile.level > paid) {
            store.setProfileInt(user, "level_paid", profile.level);
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
            Long at = earned.get(template.id);
            int progress = "total".equals(template.scope) ? Math.min(template.target, totals.getOrDefault(template.counter, 0)) : 0;
            views.add(view(template, at != null, at == null ? 0 : at, progress));
        }
        return views;
    }

    private static CareerAchievement view(AchievementTemplate template, boolean achieved, long at, int progress) {
        CareerAchievement view = new CareerAchievement();
        view.id = template.id;
        view.secret = template.secret;
        view.achieved = achieved;
        view.at = at;
        view.scope = template.scope;
        view.target = template.target;
        view.progress = achieved ? template.target : progress;
        view.coins = template.coins;
        view.xp = template.xp;
        view.cosmetic = template.cosmetic;
        if (achieved || !template.secret) {
            view.name = template.name;
            view.text = template.text;
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
