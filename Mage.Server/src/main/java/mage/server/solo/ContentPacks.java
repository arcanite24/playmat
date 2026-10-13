package mage.server.solo;

import com.google.gson.Gson;
import com.google.gson.JsonParseException;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardRepository;
import mage.server.career.CareerCampaigns;
import mage.server.career.CareerChallenges;
import mage.server.career.CareerContent;
import mage.server.career.CareerContentHooks;
import mage.server.career.CareerSetup;
import org.apache.log4j.Logger;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;
import java.util.stream.Stream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

/**
 * Content packs (S007): campaigns, puzzles, weekly challenges and roster opponents that live outside the server's
 * code, in {@code db/content-packs/<pack id>/}: a {@code pack.json} and the {@code .dck} decks it names under
 * {@code decks/}. Staff upload a pack as a zip from /admin, or drop the folder in by hand and reload.
 * <p>
 * Every pack is checked as it loads: one that fails (bad JSON, a missing deck, an id the server already has, a card
 * the database doesn't know) is skipped whole and listed with its problems. Pack content joins the built-in content
 * after it; a pack's weekly challenges run only in the weeks they name, so loading a pack never moves the rotation.
 * <p>
 * Decks in pack content are named {@code pack:<pack id>/<file>.dck}; a pack's own content may write just the file
 * name, and it's expanded as the pack loads.
 */
public final class ContentPacks {

    private static final Logger logger = Logger.getLogger(ContentPacks.class);
    private static final ContentPacks INSTANCE = new ContentPacks();
    private static final Pattern ID = Pattern.compile("[a-z0-9][a-z0-9-]{0,39}");
    private static final Pattern FILE = Pattern.compile("[A-Za-z0-9][A-Za-z0-9 _.-]{0,79}\\.dck");
    private static final Pattern WEEK = Pattern.compile("\\d{4}-W\\d{2}");
    public static final String DECK_PREFIX = "pack:";
    public static final int MAX_UPLOAD_BYTES = 4 * 1024 * 1024;
    private static final int MAX_FILES = 200;

    /** What a pack.json holds. */
    public static class Pack {
        public String id;
        public String name;
        public String version;
        public String author;
        public String text;
        public List<CareerCampaigns.Campaign> campaigns = new ArrayList<>();
        public List<CareerSetup.Puzzle> puzzles = new ArrayList<>();
        public List<PackChallenge> challenges = new ArrayList<>();
        public List<CareerContent.Opponent> opponents = new ArrayList<>();
    }

    /** a weekly challenge that runs in the week it names ("2026-W44") */
    public static class PackChallenge extends CareerChallenges.Challenge {
    }

    /** A pack as staff see it. */
    public static class ContentPackView {
        public String id;
        public String name;
        public String version;
        public String author;
        public boolean loaded;
        public int campaigns;
        public int puzzles;
        public int challenges;
        public int opponents;
        public int decks;
        public List<String> problems = new ArrayList<>();
    }

    private volatile Path root = Paths.get("db", "content-packs");
    private volatile Map<String, Pack> packs = Collections.emptyMap();
    private volatile List<ContentPackView> views = Collections.emptyList();
    private volatile boolean loaded;

    public static ContentPacks get() {
        return INSTANCE;
    }

    /** for tests: read packs from another folder */
    public synchronized void useRoot(Path root) {
        this.root = root;
        this.loaded = false;
    }

    public Path root() {
        return root;
    }

    // ---- what the career content reads

    public List<CareerCampaigns.Campaign> campaigns() {
        List<CareerCampaigns.Campaign> all = new ArrayList<>();
        loadedPacks().values().forEach(pack -> all.addAll(pack.campaigns));
        return all;
    }

    public List<CareerSetup.Puzzle> puzzles() {
        List<CareerSetup.Puzzle> all = new ArrayList<>();
        loadedPacks().values().forEach(pack -> all.addAll(pack.puzzles));
        return all;
    }

    public List<PackChallenge> challenges() {
        List<PackChallenge> all = new ArrayList<>();
        loadedPacks().values().forEach(pack -> all.addAll(pack.challenges));
        return all;
    }

    public List<CareerContent.Opponent> opponents() {
        List<CareerContent.Opponent> all = new ArrayList<>();
        loadedPacks().values().forEach(pack -> all.addAll(pack.opponents));
        return all;
    }

    public List<ContentPackView> list() {
        loadedPacks();
        return views;
    }

    /** whether a deck path names a pack deck ("campaigns/decks/pack:id/file.dck" or "pack:id/file.dck") */
    public static boolean isPackDeck(String path) {
        return path != null && path.contains(DECK_PREFIX);
    }

    /** the file a pack deck path points at, or null when it's outside every pack */
    public Path deckFile(String path) {
        String ref = path.substring(path.indexOf(DECK_PREFIX) + DECK_PREFIX.length());
        int slash = ref.indexOf('/');
        if (slash < 1) {
            return null;
        }
        String packId = ref.substring(0, slash);
        String file = ref.substring(slash + 1);
        if (!ID.matcher(packId).matches() || !FILE.matcher(file).matches()) {
            return null;
        }
        return root.resolve(packId).resolve("decks").resolve(file);
    }

    public DeckCardLists deck(String path) throws IOException {
        Path file = deckFile(path);
        if (file == null || !Files.isRegularFile(file)) {
            throw new IOException("There's no pack deck " + path);
        }
        return readDeck(file);
    }

    // ---- loading

    private Map<String, Pack> loadedPacks() {
        if (!loaded) {
            synchronized (this) {
                if (!loaded) {
                    load();
                }
            }
        }
        return packs;
    }

    /** reads every pack again; the career content picks them up on its next read */
    public synchronized List<ContentPackView> reload() {
        load();
        CareerContentHooks.reloadAll();
        return views;
    }

    private void load() {
        Map<String, Pack> good = new LinkedHashMap<>();
        List<ContentPackView> all = new ArrayList<>();
        loaded = true;
        if (!Files.isDirectory(root)) {
            packs = good;
            views = all;
            return;
        }
        List<Path> dirs = new ArrayList<>();
        try (DirectoryStream<Path> stream = Files.newDirectoryStream(root, Files::isDirectory)) {
            stream.forEach(dirs::add);
        } catch (IOException e) {
            logger.warn("Content packs can't be listed", e);
        }
        dirs.sort(Comparator.comparing(path -> path.getFileName().toString()));
        Ids taken = Ids.builtIn();
        for (Path dir : dirs) {
            ContentPackView view = new ContentPackView();
            view.id = dir.getFileName().toString();
            Pack pack = null;
            try {
                pack = read(dir, view.id, view.problems);
            } catch (IOException | JsonParseException e) {
                view.problems.add("pack.json can't be read: " + e.getMessage());
            }
            if (pack != null) {
                view.name = pack.name;
                view.version = pack.version;
                view.author = pack.author;
                view.campaigns = pack.campaigns.size();
                view.puzzles = pack.puzzles.size();
                view.challenges = pack.challenges.size();
                view.opponents = pack.opponents.size();
                view.decks = countDecks(dir);
                if (view.problems.isEmpty()) {
                    validate(dir, pack, taken, view.problems);
                }
            }
            view.loaded = pack != null && view.problems.isEmpty();
            if (view.loaded) {
                good.put(pack.id, pack);
                taken.add(pack);
            } else {
                logger.warn("Content pack " + view.id + " skipped: " + String.join("; ", view.problems));
            }
            all.add(view);
        }
        packs = Collections.unmodifiableMap(good);
        views = Collections.unmodifiableList(all);
        if (!good.isEmpty()) {
            logger.info("Content packs loaded: " + String.join(", ", good.keySet()));
        }
    }

    private static Pack read(Path dir, String folder, List<String> problems) throws IOException {
        if (!ID.matcher(folder).matches()) {
            problems.add("The folder name must be a pack id: lower-case letters, digits and dashes.");
            return null;
        }
        Path file = dir.resolve("pack.json");
        if (!Files.isRegularFile(file)) {
            problems.add("There's no pack.json.");
            return null;
        }
        Pack pack;
        try (Reader reader = Files.newBufferedReader(file, StandardCharsets.UTF_8)) {
            pack = new Gson().fromJson(reader, Pack.class);
        }
        if (pack == null) {
            problems.add("pack.json is empty.");
            return null;
        }
        if (pack.id == null || !pack.id.equals(folder)) {
            problems.add("The pack's id must match its folder (" + folder + ").");
        }
        if (blank(pack.name)) {
            problems.add("The pack needs a name.");
        }
        pack.campaigns = orEmpty(pack.campaigns);
        pack.puzzles = orEmpty(pack.puzzles);
        pack.challenges = orEmpty(pack.challenges);
        pack.opponents = orEmpty(pack.opponents);
        expandDecks(pack, folder);
        return pack;
    }

    /** a pack's content may name its decks by file name alone */
    private static void expandDecks(Pack pack, String id) {
        for (CareerCampaigns.Campaign campaign : pack.campaigns) {
            for (CareerCampaigns.Chapter chapter : orEmpty(campaign.chapters)) {
                chapter.deck = expand(chapter.deck, id);
                for (CareerCampaigns.Node node : orEmpty(chapter.nodes)) {
                    node.deck = expand(node.deck, id);
                    if (node.opponent != null) {
                        node.opponent.deck = expand(node.opponent.deck, id);
                    }
                    for (CareerCampaigns.NodeOpponent opponent : orEmpty(node.opponents)) {
                        opponent.deck = expand(opponent.deck, id);
                    }
                }
            }
        }
        for (PackChallenge challenge : pack.challenges) {
            challenge.playerDeck = expand(challenge.playerDeck, id);
            if (challenge.opponent != null) {
                challenge.opponent.deck = expand(challenge.opponent.deck, id);
            }
        }
        for (CareerContent.Opponent opponent : pack.opponents) {
            opponent.deck = expand(opponent.deck, id);
        }
    }

    private static String expand(String deck, String id) {
        if (deck == null || deck.isEmpty() || deck.startsWith(DECK_PREFIX)) {
            return deck;
        }
        return DECK_PREFIX + id + "/" + deck;
    }

    private void validate(Path dir, Pack pack, Ids taken, List<String> problems) {
        Set<String> decks = new HashSet<>();
        Set<String> ids = new HashSet<>();
        for (CareerCampaigns.Campaign campaign : pack.campaigns) {
            String where = "Campaign " + campaign.id;
            if (blank(campaign.id) || !ID.matcher(campaign.id).matches()) {
                problems.add("A campaign needs an id of lower-case letters, digits and dashes.");
                continue;
            }
            if (taken.campaigns.contains(campaign.id) || !ids.add("campaign:" + campaign.id)) {
                problems.add(where + ": the server already has a campaign with this id.");
            }
            if (blank(campaign.name)) {
                problems.add(where + " needs a name.");
            }
            if (orEmpty(campaign.chapters).isEmpty()) {
                problems.add(where + " has no chapters.");
            }
            Set<String> nodes = new HashSet<>();
            for (CareerCampaigns.Chapter chapter : orEmpty(campaign.chapters)) {
                if (blank(chapter.id)) {
                    problems.add(where + ": a chapter needs an id.");
                }
                deckRef(chapter.deck, where + ", chapter " + chapter.id, decks, problems, false);
                if (orEmpty(chapter.nodes).isEmpty()) {
                    problems.add(where + ", chapter " + chapter.id + " has no nodes.");
                }
                for (CareerCampaigns.Node node : orEmpty(chapter.nodes)) {
                    String at = where + ", node " + node.id;
                    if (blank(node.id) || !nodes.add(node.id)) {
                        problems.add(where + ": every node needs its own id (" + node.id + ").");
                    }
                    deckRef(node.deck, at, decks, problems, false);
                    if (node.opponent != null) {
                        deckRef(node.opponent.deck, at, decks, problems, true);
                        skill(node.opponent.skill, at, problems);
                        twists(node.opponent.twists, at, problems);
                    }
                    for (CareerCampaigns.NodeOpponent opponent : orEmpty(node.opponents)) {
                        deckRef(opponent.deck, at, decks, problems, true);
                        skill(opponent.skill, at, problems);
                        twists(opponent.twists, at, problems);
                    }
                    for (String required : orEmpty(node.requires)) {
                        if (!nodeIds(campaign).contains(required)) {
                            problems.add(at + " requires " + required + ", which isn't in the campaign.");
                        }
                    }
                }
            }
        }
        for (CareerSetup.Puzzle puzzle : pack.puzzles) {
            String where = "Puzzle " + puzzle.id;
            if (blank(puzzle.id)) {
                problems.add("A puzzle needs an id.");
                continue;
            }
            if (taken.puzzles.contains(puzzle.id) || !ids.add("puzzle:" + puzzle.id)) {
                problems.add(where + ": the server already has a puzzle with this id.");
            }
            if (blank(puzzle.name)) {
                problems.add(where + " needs a name.");
            }
            if (puzzle.you == null || puzzle.opponent == null) {
                problems.add(where + " needs both sides (you and opponent).");
                continue;
            }
            for (CareerSetup.CareerPuzzleSide side : new CareerSetup.CareerPuzzleSide[]{puzzle.you, puzzle.opponent}) {
                for (List<String> zone : java.util.Arrays.asList(side.hand, side.battlefield, side.graveyard, side.library)) {
                    for (String card : orEmpty(zone)) {
                        cardName(card, where, problems);
                    }
                }
            }
        }
        for (PackChallenge challenge : pack.challenges) {
            String where = "Challenge " + challenge.id;
            if (blank(challenge.id)) {
                problems.add("A challenge needs an id.");
                continue;
            }
            if (taken.challenges.contains(challenge.id) || !ids.add("challenge:" + challenge.id)) {
                problems.add(where + ": the server already has a challenge with this id.");
            }
            if (challenge.week == null || !WEEK.matcher(challenge.week).matches()) {
                problems.add(where + " needs the week it runs, like 2026-W44.");
            } else if (taken.weeks.contains(challenge.week) || !ids.add("week:" + challenge.week)) {
                problems.add(where + ": another pack's challenge already runs in " + challenge.week + ".");
            }
            if (blank(challenge.name)) {
                problems.add(where + " needs a name.");
            }
            deckRef(challenge.playerDeck, where, decks, problems, true);
            if (challenge.opponent == null) {
                problems.add(where + " needs an opponent.");
            } else {
                deckRef(challenge.opponent.deck, where, decks, problems, true);
                skill(challenge.opponent.skill, where, problems);
                twists(challenge.opponent.twists, where, problems);
            }
        }
        for (CareerContent.Opponent opponent : pack.opponents) {
            String where = "Opponent " + opponent.id;
            if (blank(opponent.id)) {
                problems.add("An opponent needs an id.");
                continue;
            }
            if (taken.opponents.contains(opponent.id) || !ids.add("opponent:" + opponent.id)) {
                problems.add(where + ": the server already has an opponent with this id.");
            }
            if (blank(opponent.name)) {
                problems.add(where + " needs a name.");
            }
            if (!taken.tiers.contains(opponent.tier)) {
                problems.add(where + ": there's no tier " + opponent.tier + ".");
            }
            if (opponent.skill != null) {
                skill(opponent.skill, where, problems);
            }
            deckRef(opponent.deck, where, decks, problems, true);
        }
        for (String deck : decks) {
            Path file = deckFile(deck);
            if (file == null || !file.startsWith(dir.resolve("decks"))) {
                problems.add("Deck " + deck + " must be one of this pack's own decks.");
                continue;
            }
            if (!Files.isRegularFile(file)) {
                problems.add("Deck " + file.getFileName() + " is missing from decks/.");
                continue;
            }
            try {
                DeckCardLists list = readDeck(file);
                if (list.getCards().isEmpty()) {
                    problems.add("Deck " + file.getFileName() + " has no cards.");
                }
            } catch (IOException e) {
                problems.add("Deck " + file.getFileName() + ": " + e.getMessage());
            }
        }
        if (pack.campaigns.isEmpty() && pack.puzzles.isEmpty() && pack.challenges.isEmpty() && pack.opponents.isEmpty()) {
            problems.add("The pack has nothing in it.");
        }
    }

    private static Set<String> nodeIds(CareerCampaigns.Campaign campaign) {
        Set<String> ids = new HashSet<>();
        for (CareerCampaigns.Chapter chapter : orEmpty(campaign.chapters)) {
            for (CareerCampaigns.Node node : orEmpty(chapter.nodes)) {
                ids.add(node.id);
            }
        }
        return ids;
    }

    private static void deckRef(String deck, String where, Set<String> decks, List<String> problems, boolean required) {
        if (deck == null || deck.isEmpty()) {
            if (required) {
                problems.add(where + " needs a deck.");
            }
            return;
        }
        if (!deck.startsWith(DECK_PREFIX)) {
            problems.add(where + ": deck " + deck + " isn't a pack deck.");
            return;
        }
        decks.add(deck);
    }

    private static void twists(CareerSetup.CareerTwists twists, String where, List<String> problems) {
        if (twists == null) {
            return;
        }
        for (String card : orEmpty(twists.permanents)) {
            cardName(CareerSetupNames.name(card), where, problems);
        }
        if (twists.emblem != null) {
            cardName(twists.emblem, where, problems);
        }
        for (CareerSetup.TimedTwist timed : orEmpty(twists.timed)) {
            String problem = timed.problem();
            if (problem != null) {
                problems.add(where + ": " + problem + ".");
            } else if (timed.then != null) {
                twists(timed.then, where, problems);
            }
            if (timed.card != null) {
                cardName(timed.card, where, problems);
            }
        }
    }

    /** "Name|tapped" to "Name" */
    private static final class CareerSetupNames {
        static String name(String entry) {
            int bar = entry == null ? -1 : entry.indexOf('|');
            return entry == null ? null : (bar < 0 ? entry : entry.substring(0, bar)).trim();
        }
    }

    private static void skill(int skill, String where, List<String> problems) {
        if (skill < 1 || skill > 10) {
            problems.add(where + ": AI skill is 1 to 10.");
        }
    }

    private static void cardName(String card, String where, List<String> problems) {
        if (blank(card)) {
            problems.add(where + ": a card has no name.");
            return;
        }
        if (!CardRepository.instance.findCards("Plains").isEmpty() && CardRepository.instance.findCards(card).isEmpty()) {
            problems.add(where + ": there's no card named " + card + ".");
        }
    }

    private static DeckCardLists readDeck(Path file) throws IOException {
        return CareerContent.readDeckFile(file);
    }

    private static int countDecks(Path dir) {
        Path decks = dir.resolve("decks");
        if (!Files.isDirectory(decks)) {
            return 0;
        }
        try (Stream<Path> files = Files.list(decks)) {
            return (int) files.filter(file -> file.toString().endsWith(".dck")).count();
        } catch (IOException e) {
            return 0;
        }
    }

    // ---- staff changes

    /**
     * Unpacks an uploaded zip as a pack, replacing one with the same id, and reloads. The zip holds pack.json and
     * decks/*.dck, at its top or in one folder; nothing else is taken.
     *
     * @return the pack's view after the reload
     * @throws IllegalArgumentException when the zip isn't a pack
     */
    public synchronized ContentPackView install(byte[] zip) throws IOException {
        if (zip == null || zip.length == 0 || zip.length > MAX_UPLOAD_BYTES) {
            throw new IllegalArgumentException("A pack is a zip of up to " + (MAX_UPLOAD_BYTES / 1024 / 1024) + " MB.");
        }
        Map<String, byte[]> files = new LinkedHashMap<>();
        long total = 0;
        try (ZipInputStream in = new ZipInputStream(new ByteArrayInputStream(zip))) {
            ZipEntry entry;
            while ((entry = in.getNextEntry()) != null) {
                if (entry.isDirectory()) {
                    continue;
                }
                String name = entry.getName().replace('\\', '/');
                byte[] data = readLimited(in, MAX_UPLOAD_BYTES - total);
                total += data.length;
                files.put(name, data);
                if (files.size() > MAX_FILES) {
                    throw new IllegalArgumentException("A pack holds at most " + MAX_FILES + " files.");
                }
            }
        }
        // pack.json at the top, or in one folder
        String base = null;
        for (String name : files.keySet()) {
            if (name.equals("pack.json") || name.endsWith("/pack.json") && name.indexOf('/') == name.lastIndexOf('/')) {
                base = name.substring(0, name.length() - "pack.json".length());
                break;
            }
        }
        if (base == null) {
            throw new IllegalArgumentException("The zip has no pack.json.");
        }
        Pack pack;
        try {
            pack = new Gson().fromJson(new String(files.get(base + "pack.json"), StandardCharsets.UTF_8), Pack.class);
        } catch (JsonParseException e) {
            throw new IllegalArgumentException("pack.json can't be read: " + e.getMessage());
        }
        if (pack == null || pack.id == null || !ID.matcher(pack.id).matches()) {
            throw new IllegalArgumentException("pack.json needs an id of lower-case letters, digits and dashes.");
        }
        Files.createDirectories(root);
        Path staging = Files.createTempDirectory(root, ".upload-");
        try {
            Files.createDirectories(staging.resolve("decks"));
            for (Map.Entry<String, byte[]> file : files.entrySet()) {
                if (!file.getKey().startsWith(base)) {
                    continue;
                }
                String name = file.getKey().substring(base.length());
                Path target;
                if (name.equals("pack.json")) {
                    target = staging.resolve("pack.json");
                } else if (name.startsWith("decks/") && FILE.matcher(name.substring("decks/".length())).matches()) {
                    target = staging.resolve("decks").resolve(name.substring("decks/".length()));
                } else {
                    continue;
                }
                Files.write(target, file.getValue());
            }
            Path dir = root.resolve(pack.id);
            if (Files.exists(dir)) {
                deleteTree(dir);
            }
            Files.move(staging, dir, StandardCopyOption.ATOMIC_MOVE);
        } finally {
            if (Files.exists(staging)) {
                deleteTree(staging);
            }
        }
        reload();
        for (ContentPackView view : views) {
            if (view.id.equals(pack.id)) {
                return view;
            }
        }
        throw new IOException("The pack was written but isn't listed");
    }

    /** @return false when there's no such pack */
    public synchronized boolean remove(String id) throws IOException {
        if (id == null || !ID.matcher(id).matches()) {
            return false;
        }
        Path dir = root.resolve(id);
        if (!Files.isDirectory(dir)) {
            return false;
        }
        deleteTree(dir);
        reload();
        return true;
    }

    private static byte[] readLimited(ZipInputStream in, long left) throws IOException {
        java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
        byte[] buffer = new byte[8192];
        int read;
        while ((read = in.read(buffer)) > 0) {
            out.write(buffer, 0, read);
            if (out.size() > left) {
                throw new IllegalArgumentException("The pack unpacks to more than " + (MAX_UPLOAD_BYTES / 1024 / 1024) + " MB.");
            }
        }
        return out.toByteArray();
    }

    private static void deleteTree(Path dir) throws IOException {
        try (Stream<Path> paths = Files.walk(dir)) {
            List<Path> all = new ArrayList<>();
            paths.forEach(all::add);
            Collections.reverse(all);
            for (Path path : all) {
                Files.deleteIfExists(path);
            }
        }
    }

    private static boolean blank(String text) {
        return text == null || text.trim().isEmpty();
    }

    private static <T> List<T> orEmpty(List<T> list) {
        return list == null ? Collections.emptyList() : list;
    }

    /** the ids the server's own content uses, and those of packs loaded before */
    private static final class Ids {
        final Set<String> campaigns = new HashSet<>();
        final Set<String> puzzles = new HashSet<>();
        final Set<String> challenges = new HashSet<>();
        final Set<String> opponents = new HashSet<>();
        final Set<String> weeks = new HashSet<>();
        final Set<Integer> tiers = new HashSet<>();

        static Ids builtIn() {
            Ids ids = new Ids();
            try {
                ids.campaigns.addAll(CareerContentHooks.builtInCampaignIds());
                ids.puzzles.addAll(CareerContentHooks.builtInPuzzleIds());
                ids.challenges.addAll(CareerContentHooks.builtInChallengeIds());
                ids.opponents.addAll(CareerContentHooks.builtInOpponentIds());
                ids.tiers.addAll(CareerContentHooks.tiers());
            } catch (RuntimeException e) {
                logger.warn("Content packs: the built-in content can't be read to check ids against", e);
            }
            return ids;
        }

        void add(Pack pack) {
            pack.campaigns.forEach(c -> campaigns.add(c.id));
            pack.puzzles.forEach(p -> puzzles.add(p.id));
            pack.challenges.forEach(c -> {
                challenges.add(c.id);
                weeks.add(c.week);
            });
            pack.opponents.forEach(o -> opponents.add(o.id));
        }
    }
}
