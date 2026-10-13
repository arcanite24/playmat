package mage.server.career;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Campaigns: chapters of duels and choices, as data (resources/career/campaigns). A node opens when every node it
 * requires is done ("node" or "node:option" for a particular choice); a duel's reward is paid the first time it's won.
 * The decks live next to the campaign files, and a chapter's deck is what the player plays its duels with.
 */
public final class CareerCampaigns {

    private static final String ROOT = "/career/campaigns/";

    // ---- content

    public static class Campaign {
        public String id;
        public String name;
        public String text;
        /** a client-side game played before the first chapter (the guided first game) */
        public String prologue;
        /** a format school (CURRICULUM.md): the academy lists it, and it opens by its {@link #requires} */
        public boolean school;
        /** the format a school teaches, as players call it */
        public String format;
        /** the school's place on the academy's wall */
        public int order;
        /** WUBRG letters for the school's crest */
        public String colors;
        /** one line on the academy hall */
        public String summary;
        /** the deck type every duel is played under (else "Constructed - Freeform") */
        public String deckType;
        /** the game type every duel is played as (else "Two Player Duel") */
        public String gameType;
        /** "campaign@n" (n of its chapters' bosses beaten) or "campaign" (all of it), "a|b" for either; all must hold */
        public List<String> requires;
        public List<Chapter> chapters;
    }

    public static class Chapter {
        public String id;
        public String name;
        /** WUBRG letters */
        public String color;
        public String text;
        /** the deck the player plays this chapter's duels with */
        public String deck;
        public String deckName;
        /** the shop set the chapter's boss opens */
        public String unlockSet;
        public List<Node> nodes;
    }

    public static class Node {
        public String id;
        /** "duel", "choice" or "trial" */
        public String type;
        public String name;
        public String text;
        public boolean boss;
        public NodeOpponent opponent;
        /** a pod's other AI seats */
        public List<NodeOpponent> opponents;
        /** this duel's loaner, else the chapter's deck */
        public String deck;
        public String deckName;
        /** overrides the campaign's */
        public String deckType;
        public String gameType;
        /** games to win the match: 2 is a best of three */
        public int winsNeeded;
        /** what the player should know going in */
        public CareerLesson lesson;
        /** shown once each in the game, when their moment comes */
        public List<CareerTip> tips;
        /** shown while sideboarding between games */
        public String sideboard;
        /** a trial's position */
        public CareerTrial trial;
        /** a nudge for a player stuck on a trial */
        public String hint;
        public List<String> requires;
        public CareerReward reward;
        public List<CareerOption> options;
        /** what the opponent says before the duel */
        public String before;
        /** what the opponent says once beaten */
        public String after;
    }

    public static class NodeOpponent {
        public String name;
        public String deck;
        public int skill;
        public String aiType;
        public CareerSetup.CareerTwists twists;
    }

    /** The card shown before a duel: a title, a few sentences and up to four key cards. */
    public static class CareerLesson {
        public String title;
        public String text;
        public List<String> cards;
    }

    /** A coach line for a moment in the game; {@code on} is a trigger from CURRICULUM.md ("cast:Card", "attack", ...). */
    public static class CareerTip {
        public String on;
        public String title;
        public String text;
    }

    /** A set position: win this turn, or survive the opponent's turn. */
    public static class CareerTrial {
        /** "win" or "survive" */
        public String objective;
        public CareerSetup.CareerPuzzleSide you;
        public CareerSetup.CareerPuzzleSide opponent;
    }

    public static class CareerOption {
        public String id;
        public String text;
        public CareerReward reward;
    }

    /** What finishing a node gives, the first time. */
    public static class CareerReward {
        public int coins;
        public int xp;
        public int packs;
        /** a wildcard of this rarity */
        public String wildcard;
        /** the chapter's deck joins the collection */
        public boolean deckCards;
        /** this set's packs appear in the shop */
        public String unlockSet;
        /** sleeves, titles, avatars or playmats */
        public List<CareerProgress.CareerCosmetic> cosmetics;
    }

    // ---- views

    public static class CareerCampaign {
        public String id;
        public String name;
        public String text;
        public String prologue;
        public boolean school;
        public String format;
        public int order;
        public String colors;
        public String summary;
        public String deckType;
        public String gameType;
        public List<String> requires;
        /** whether the campaign is open; a school opens by its requirements */
        public boolean open = true;
        /** what's still needed, one requirement a line, when it isn't open */
        public List<String> missing = new ArrayList<>();
        /** chapters whose boss is beaten */
        public int bosses;
        public List<CareerChapter> chapters = new ArrayList<>();
        public int done;
        public int total;
    }

    public static class CareerChapter {
        public String id;
        public String name;
        public String color;
        public String text;
        public String deckName;
        public String unlockSet;
        public boolean done;
        public List<CareerNode> nodes = new ArrayList<>();
    }

    public static class CareerNode {
        public String id;
        public String type;
        public String name;
        public String text;
        public boolean boss;
        /** "locked", "open" or "done" */
        public String state;
        /** the option taken, for a finished choice */
        public String choice;
        public List<String> requires;
        public CareerReward reward;
        public List<CareerOption> options;
        public int skill;
        public CareerSetup.CareerTwists twists;
        /** the card whose art is the opponent's portrait */
        public CareerContent.CareerCover cover;
        /** what the opponent says before the duel */
        public String before;
        /** what the opponent says once beaten */
        public String after;
        /** the opponent's name (the first seat's, in a pod) */
        public String opponentName;
        /** a pod's other seats */
        public List<CareerSeat> others;
        /** the deck the player plays here */
        public String deckName;
        public String gameType;
        public int winsNeeded;
        public CareerLesson lesson;
        public List<CareerTip> tips;
        public String sideboard;
        public String hint;
        /** a trial's objective, "win" or "survive" */
        public String objective;
    }

    /** One more AI at a pod. */
    public static class CareerSeat {
        public String name;
        public int skill;
        public CareerContent.CareerCover cover;
    }

    /** One AI seat of a duel or a pod. */
    public static class DuelSeat {
        public String name;
        public int skill;
        public String aiType;
        public DeckCardLists deck;
    }

    /** What a played duel needs: who, with which decks, and how the game starts. */
    public static class DuelPlan {
        public String key;
        public String opponentName;
        public int skill;
        public String aiType;
        public DeckCardLists playerDeck;
        public DeckCardLists opponentDeck;
        public CareerSetup setup;
        /** "campaign", or "trial" for a set position (paid like a puzzle: a turn, not a game) */
        public String kind = "campaign";
        public String deckType = "Constructed - Freeform";
        public String gameType = "Two Player Duel";
        public int winsNeeded = 1;
        /** every AI seat, the first being {@link #opponentName}'s */
        public List<DuelSeat> seats = new ArrayList<>();
    }

    private static volatile CareerCampaigns instance;

    private final Map<String, Campaign> campaigns = new LinkedHashMap<>();

    private CareerCampaigns() throws IOException {
        campaigns.putAll(readBuiltIn());
        for (Campaign campaign : mage.server.solo.ContentPacks.get().campaigns()) {
            campaigns.putIfAbsent(campaign.id, campaign);
        }
    }

    /** the campaigns the server ships with */
    static Map<String, Campaign> readBuiltIn() throws IOException {
        Gson gson = new Gson();
        Map<String, Campaign> read = new LinkedHashMap<>();
        List<String> ids;
        try (Reader reader = resource("index.json")) {
            ids = gson.fromJson(reader, new TypeToken<List<String>>() {
            }.getType());
        }
        for (String id : ids) {
            try (Reader reader = resource(id + ".json")) {
                Campaign campaign = gson.fromJson(reader, Campaign.class);
                read.put(campaign.id, campaign);
            }
        }
        return read;
    }

    /** content packs changed: read everything again on next use */
    static void forget() {
        instance = null;
    }

    private static Reader resource(String name) throws IOException {
        InputStream stream = CareerCampaigns.class.getResourceAsStream(ROOT + name);
        if (stream == null) {
            throw new IOException("Missing campaign resource " + name);
        }
        return new InputStreamReader(stream, StandardCharsets.UTF_8);
    }

    public static CareerCampaigns get() {
        CareerCampaigns current = instance;
        if (current == null) {
            synchronized (CareerCampaigns.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerCampaigns();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("Campaigns can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    public Map<String, Campaign> campaigns() {
        return Collections.unmodifiableMap(campaigns);
    }

    public DeckCardLists deck(String file) throws IOException {
        return CareerContent.deck("campaigns/decks/" + file);
    }

    static String key(String campaign, String node) {
        return campaign + "/" + node;
    }

    private Campaign campaign(String id) throws CareerService.CareerException {
        Campaign campaign = campaigns.get(id);
        if (campaign == null) {
            throw new CareerService.CareerException("There is no campaign " + id + ".");
        }
        return campaign;
    }

    private static Chapter chapterOf(Campaign campaign, String nodeId) {
        for (Chapter chapter : campaign.chapters) {
            for (Node node : chapter.nodes) {
                if (node.id.equals(nodeId)) {
                    return chapter;
                }
            }
        }
        return null;
    }

    private static Node node(Chapter chapter, String nodeId) {
        return chapter.nodes.stream().filter(node -> node.id.equals(nodeId)).findFirst().orElse(null);
    }

    /** every requirement is met: "node" is done, or "node:option" was chosen */
    static boolean open(Node node, Map<String, String> done) {
        if (node.requires == null) {
            return true;
        }
        for (String requirement : node.requires) {
            int colon = requirement.indexOf(':');
            String id = colon < 0 ? requirement : requirement.substring(0, colon);
            if (!done.containsKey(id) || (colon >= 0 && !requirement.substring(colon + 1).equals(done.get(id)))) {
                return false;
            }
        }
        return true;
    }

    /** the node whose win finishes a chapter: its boss, else its last node */
    private static Node bossOf(Chapter chapter) {
        if (chapter.nodes == null || chapter.nodes.isEmpty()) {
            return null;
        }
        return chapter.nodes.stream().filter(node -> node.boss).findFirst().orElse(chapter.nodes.get(chapter.nodes.size() - 1));
    }

    /** chapters whose boss is beaten */
    static int bosses(Campaign campaign, Map<String, String> done) {
        int beaten = 0;
        for (Chapter chapter : campaign.chapters) {
            Node boss = bossOf(chapter);
            if (boss != null && done.containsKey(boss.id)) {
                beaten++;
            }
        }
        return beaten;
    }

    private static boolean finished(Campaign campaign, Map<String, String> done) {
        for (Chapter chapter : campaign.chapters) {
            for (Node node : chapter.nodes) {
                if (!done.containsKey(node.id)) {
                    return false;
                }
            }
        }
        return !campaign.chapters.isEmpty();
    }

    /**
     * The campaign's requirements that don't hold: each is "campaign@n" (n of its bosses beaten), "campaign" (all of
     * it), or alternatives joined by "|".
     */
    List<String> missing(Campaign campaign, Map<String, Map<String, String>> progress) {
        List<String> missing = new ArrayList<>();
        if (campaign.requires == null) {
            return missing;
        }
        for (String requirement : campaign.requires) {
            boolean held = false;
            for (String alternative : requirement.split("\\|")) {
                int at = alternative.indexOf('@');
                String id = (at < 0 ? alternative : alternative.substring(0, at)).trim();
                Campaign other = campaigns.get(id);
                if (other == null) {
                    continue;
                }
                Map<String, String> done = progress.getOrDefault(id, Collections.emptyMap());
                if (at < 0 ? finished(other, done) : bosses(other, done) >= Integer.parseInt(alternative.substring(at + 1).trim())) {
                    held = true;
                    break;
                }
            }
            if (!held) {
                missing.add(requirement);
            }
        }
        return missing;
    }

    private Map<String, Map<String, String>> progress(CareerStore store, String user) throws SQLException {
        Map<String, Map<String, String>> progress = new LinkedHashMap<>();
        for (String id : campaigns.keySet()) {
            progress.put(id, store.campaignNodes(user, id));
        }
        return progress;
    }

    private void checkOpen(CareerStore store, String user, Campaign campaign) throws SQLException, CareerService.CareerException {
        if (campaign.requires != null && !missing(campaign, progress(store, user)).isEmpty()) {
            throw new CareerService.CareerException(campaign.name + " isn't open yet.");
        }
    }

    private static String deckOf(Chapter chapter, Node node) {
        return node.deck != null ? node.deck : chapter.deck;
    }

    private static String or(String first, String second, String fallback) {
        return first != null ? first : second != null ? second : fallback;
    }

    // ---- reading progress

    public List<CareerCampaign> campaigns(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        Map<String, Map<String, String>> progress = progress(store, user);
        List<CareerCampaign> views = new ArrayList<>();
        for (Campaign campaign : campaigns.values()) {
            Map<String, String> done = progress.get(campaign.id);
            CareerCampaign view = new CareerCampaign();
            view.id = campaign.id;
            view.name = campaign.name;
            view.text = campaign.text;
            view.prologue = campaign.prologue;
            view.school = campaign.school;
            view.format = campaign.format;
            view.order = campaign.order;
            view.colors = campaign.colors;
            view.summary = campaign.summary;
            view.deckType = campaign.deckType;
            view.gameType = campaign.gameType;
            view.requires = campaign.requires;
            view.missing = missing(campaign, progress);
            view.open = view.missing.isEmpty();
            view.bosses = bosses(campaign, done);
            for (Chapter chapter : campaign.chapters) {
                CareerChapter chapterView = new CareerChapter();
                chapterView.id = chapter.id;
                chapterView.name = chapter.name;
                chapterView.color = chapter.color;
                chapterView.text = chapter.text;
                chapterView.deckName = chapter.deckName;
                chapterView.unlockSet = chapter.unlockSet;
                boolean all = true;
                for (Node node : chapter.nodes) {
                    CareerNode nodeView = new CareerNode();
                    nodeView.id = node.id;
                    nodeView.type = node.type;
                    nodeView.name = node.name;
                    nodeView.text = node.text;
                    nodeView.boss = node.boss;
                    nodeView.requires = node.requires;
                    nodeView.reward = node.reward;
                    nodeView.options = node.options;
                    nodeView.lesson = node.lesson;
                    nodeView.tips = node.tips;
                    nodeView.sideboard = node.sideboard;
                    nodeView.hint = node.hint;
                    if (node.opponent != null) {
                        nodeView.skill = node.opponent.skill;
                        nodeView.twists = node.opponent.twists;
                        nodeView.cover = CareerContent.coverOf("campaigns/decks/" + node.opponent.deck);
                        nodeView.opponentName = node.opponent.name;
                        nodeView.before = node.before;
                        nodeView.after = node.after;
                        nodeView.deckName = node.deck != null ? node.deckName : chapter.deckName;
                        nodeView.gameType = or(node.gameType, campaign.gameType, "Two Player Duel");
                        nodeView.winsNeeded = Math.max(1, node.winsNeeded);
                    }
                    if (node.opponents != null && !node.opponents.isEmpty()) {
                        nodeView.others = new ArrayList<>();
                        for (NodeOpponent other : node.opponents) {
                            CareerSeat seat = new CareerSeat();
                            seat.name = other.name;
                            seat.skill = other.skill;
                            seat.cover = CareerContent.coverOf("campaigns/decks/" + other.deck);
                            nodeView.others.add(seat);
                        }
                    }
                    if (node.trial != null) {
                        nodeView.objective = node.trial.objective == null ? "win" : node.trial.objective;
                        if (node.lesson != null && node.lesson.cards != null && !node.lesson.cards.isEmpty()) {
                            nodeView.cover = CareerContent.coverOfCard(node.lesson.cards.get(0));
                        }
                    }
                    if (done.containsKey(node.id)) {
                        nodeView.state = "done";
                        nodeView.choice = done.get(node.id).isEmpty() ? null : done.get(node.id);
                        view.done++;
                    } else {
                        nodeView.state = view.open && open(node, done) ? "open" : "locked";
                        all = false;
                    }
                    view.total++;
                    chapterView.nodes.add(nodeView);
                }
                chapterView.done = all;
                view.chapters.add(chapterView);
            }
            views.add(view);
        }
        return views;
    }

    // ---- playing

    /** a duel or trial the player may start now (open, or done and played again) */
    public DuelPlan plan(String user, String campaignId, String nodeId) throws SQLException, CareerService.CareerException, IOException {
        Campaign campaign = campaign(campaignId);
        Chapter chapter = chapterOf(campaign, nodeId);
        Node node = chapter == null ? null : node(chapter, nodeId);
        boolean trial = node != null && "trial".equals(node.type) && node.trial != null;
        if (node == null || !(trial || "duel".equals(node.type) && node.opponent != null)) {
            throw new CareerService.CareerException("There is no duel " + nodeId + " in " + campaign.name + ".");
        }
        CareerStore store = CareerStore.get();
        checkOpen(store, user, campaign);
        Map<String, String> done = store.campaignNodes(user, campaignId);
        if (!done.containsKey(node.id) && !open(node, done)) {
            throw new CareerService.CareerException(node.name + " isn't open yet.");
        }
        DuelPlan plan = new DuelPlan();
        plan.key = key(campaignId, nodeId);
        if (trial) {
            // a set position on placeholder decks: the trial replaces every card when the game starts
            plan.kind = "trial";
            plan.opponentName = node.opponent != null ? node.opponent.name : "Trial";
            plan.skill = node.opponent != null ? Math.max(1, node.opponent.skill) : 4;
            plan.playerDeck = CareerPuzzles.placeholderDeckFor("Plains");
            plan.opponentDeck = CareerPuzzles.placeholderDeckFor("Swamp");
            CareerSetup.Puzzle puzzle = new CareerSetup.Puzzle();
            puzzle.id = plan.key;
            puzzle.name = node.name;
            puzzle.text = node.text;
            puzzle.hint = node.hint;
            puzzle.you = node.trial.you;
            puzzle.opponent = node.trial.opponent;
            puzzle.objective = node.trial.objective;
            plan.setup = CareerSetup.ofPuzzle(puzzle);
        } else {
            plan.opponentName = node.opponent.name;
            plan.skill = node.opponent.skill;
            plan.aiType = node.opponent.aiType;
            plan.playerDeck = deck(deckOf(chapter, node));
            plan.opponentDeck = deck(node.opponent.deck);
            plan.setup = CareerSetup.ofTwists(node.opponent.twists);
            plan.deckType = or(node.deckType, campaign.deckType, plan.deckType);
            plan.gameType = or(node.gameType, campaign.gameType, plan.gameType);
            plan.winsNeeded = Math.max(1, node.winsNeeded);
        }
        plan.seats.add(seat(plan.opponentName, plan.skill, plan.aiType, plan.opponentDeck));
        if (!trial && node.opponents != null && !node.opponents.isEmpty()) {
            for (NodeOpponent other : node.opponents) {
                plan.seats.add(seat(other.name, other.skill, other.aiType, deck(other.deck)));
            }
            // once the player is out of a pod, the AI players don't play the game out
            plan.setup = CareerSetup.pod(plan.setup);
        }
        return plan;
    }

    private static DuelSeat seat(String name, int skill, String aiType, DeckCardLists deck) {
        DuelSeat seat = new DuelSeat();
        seat.name = name;
        seat.skill = skill;
        seat.aiType = aiType;
        seat.deck = deck;
        return seat;
    }

    /** the deck the player plays at a duel, for reading before it */
    public DeckCardLists playerDeck(String campaignId, String nodeId) throws CareerService.CareerException, IOException {
        Campaign campaign = campaign(campaignId);
        Chapter chapter = chapterOf(campaign, nodeId);
        Node node = chapter == null ? null : node(chapter, nodeId);
        String file = node == null ? null : deckOf(chapter, node);
        if (file == null || !"duel".equals(node.type)) {
            throw new CareerService.CareerException("There is no duel " + nodeId + " in " + campaign.name + ".");
        }
        return deck(file);
    }

    /** takes an option at a choice node, once, and pays it */
    public CareerProgress.CareerGameResult choose(String user, String campaignId, String nodeId, String optionId)
            throws SQLException, CareerService.CareerException {
        Campaign campaign = campaign(campaignId);
        Chapter chapter = chapterOf(campaign, nodeId);
        Node node = chapter == null ? null : node(chapter, nodeId);
        if (node == null || !"choice".equals(node.type) || node.options == null) {
            throw new CareerService.CareerException("There is no choice " + nodeId + " in " + campaign.name + ".");
        }
        CareerOption option = node.options.stream().filter(candidate -> candidate.id.equals(optionId)).findFirst().orElse(null);
        if (option == null) {
            throw new CareerService.CareerException("That isn't one of the options.");
        }
        CareerStore store = CareerStore.get();
        try {
            return store.transaction(() -> {
                CareerProfile profile = store.profile(user);
                if (profile == null) {
                    throw new SQLException(new CareerService.CareerException("Open a Career first."));
                }
                checkOpenIn(store, user, campaign);
                Map<String, String> done = store.campaignNodes(user, campaignId);
                if (done.containsKey(node.id)) {
                    throw new SQLException(new CareerService.CareerException("You've already chosen here."));
                }
                if (!open(node, done)) {
                    throw new SQLException(new CareerService.CareerException(node.name + " isn't open yet."));
                }
                long now = System.currentTimeMillis();
                store.finishCampaignNode(user, campaignId, node.id, option.id, now);
                CareerProgress.CareerGameResult result = new CareerProgress.CareerGameResult();
                result.gameKey = key(campaignId, node.id);
                result.career = true;
                result.mode = new CareerProgress.CareerModeResult("campaign", node.name);
                result.mode.ref = campaignId + "/" + node.id;
                pay(store, user, chapter, option.reward, result, now);
                CareerProgress.get().levelsIn(store, user, profile.xp, result, now);
                return result;
            });
        } catch (SQLException e) {
            if (e.getCause() instanceof CareerService.CareerException) {
                throw (CareerService.CareerException) e.getCause();
            }
            throw e;
        }
    }

    private void checkOpenIn(CareerStore store, String user, Campaign campaign) throws SQLException {
        try {
            checkOpen(store, user, campaign);
        } catch (CareerService.CareerException e) {
            throw new SQLException(e);
        }
    }

    /**
     * A campaign duel ended (inside the payout transaction): the first win finishes the node and pays its reward.
     */
    void finished(CareerStore store, String user, String key, boolean won, CareerProgress.CareerGameResult result, long now) throws SQLException {
        int slash = key.indexOf('/');
        Campaign campaign = campaigns.get(key.substring(0, slash));
        String nodeId = key.substring(slash + 1);
        Chapter chapter = campaign == null ? null : chapterOf(campaign, nodeId);
        Node node = chapter == null ? null : node(chapter, nodeId);
        if (node == null) {
            return;
        }
        boolean trial = "trial".equals(node.type);
        result.mode = new CareerProgress.CareerModeResult("campaign", node.name);
        // the client finds the node's closing line by it
        result.mode.ref = campaign.id + "/" + node.id;
        result.mode.trial = trial;
        if (!won) {
            result.mode.lines.add(trial ? "Not this time. The trial stays open; try again any time."
                    : "Lost to " + node.name + ". The duel stays open; try again any time.");
            return;
        }
        if (!store.finishCampaignNode(user, campaign.id, node.id, null, now)) {
            result.mode.lines.add(trial ? "Passed " + node.name + " again." : "Beat " + node.name + " again.");
            return;
        }
        result.mode.lines.add(trial ? "Passed " + node.name + "." : "Beat " + node.name + (node.boss ? ", the chapter's boss." : "."));
        pay(store, user, chapter, node.reward, result, now);
        if (campaign.school && finished(campaign, store.campaignNodes(user, campaign.id))) {
            store.addStat(user, "schools_graduated", 1);
            result.mode.lines.add("You graduated from " + campaign.name + ".");
        }
    }

    private void pay(CareerStore store, String user, Chapter chapter, CareerReward reward, CareerProgress.CareerGameResult result, long now) throws SQLException {
        if (reward == null) {
            return;
        }
        CareerProgress.CareerModeResult mode = result.mode;
        if (reward.coins > 0) {
            store.addCoins(user, reward.coins);
            result.coins += reward.coins;
            mode.coins += reward.coins;
        }
        if (reward.xp > 0) {
            store.addProfileInt(user, "xp", reward.xp);
            result.xp += reward.xp;
            mode.xp += reward.xp;
        }
        if (reward.packs > 0) {
            store.addProfileInt(user, "pack_tokens", reward.packs);
            result.packs += reward.packs;
            mode.lines.add(reward.packs == 1 ? "A free pack." : reward.packs + " free packs.");
        }
        if (reward.wildcard != null) {
            store.addWildcard(user, reward.wildcard, 1);
            mode.lines.add("A " + reward.wildcard + " wildcard.");
        }
        if (reward.deckCards && chapter != null) {
            try {
                DeckCardLists deck = deck(chapter.deck);
                List<CareerCard> cards = new ArrayList<>();
                for (DeckCardInfo info : CareerContent.allCards(deck)) {
                    for (int i = 0; i < info.getAmount(); i++) {
                        CareerCard card = CareerService.get().resolveCard(info);
                        if (card != null) {
                            cards.add(card);
                        }
                    }
                }
                CareerService.get().addCards(store, user, cards);
                mode.lines.add(chapter.deckName + " joins your collection.");
            } catch (IOException e) {
                throw new SQLException("The chapter deck can't be read", e);
            }
        }
        if (reward.cosmetics != null) {
            for (CareerProgress.CareerCosmetic cosmetic : reward.cosmetics) {
                if (store.addUnlock(user, cosmetic.kind, cosmetic.id, now)) {
                    mode.cosmetics.add(cosmetic);
                }
            }
        }
        if (reward.unlockSet != null && store.addSetUnlock(user, reward.unlockSet, now)) {
            mode.lines.add("Packs of " + setName(reward.unlockSet) + " are now in the shop.");
            mode.unlockedSet = reward.unlockSet;
        }
    }

    private static String setName(String code) {
        mage.cards.ExpansionSet set = mage.cards.Sets.findSet(code);
        return set == null ? code : set.getName();
    }

    /** every set some campaign chapter opens */
    public List<String> unlockableSets() {
        List<String> sets = new ArrayList<>();
        for (Campaign campaign : campaigns.values()) {
            for (Chapter chapter : campaign.chapters) {
                collect(sets, chapter.unlockSet);
                for (Node node : chapter.nodes) {
                    if (node.reward != null) {
                        collect(sets, node.reward.unlockSet);
                    }
                }
            }
        }
        return sets;
    }

    private static void collect(List<String> sets, String set) {
        if (set != null && !sets.contains(set)) {
            sets.add(set);
        }
    }
}
