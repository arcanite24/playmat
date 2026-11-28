package mage.server.ladder;

import com.google.gson.Gson;
import mage.cards.decks.DeckCardLists;
import mage.server.career.CareerCampaigns;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * The ladder's bots (#430), from resources/ladder/bots.json: a school deck at the AI skill the Rust host plays, with a
 * persona and the rating its calibration measured (Mage.Server/ladder-src/roster.py makes the file, ladder-calibrate
 * measures it). Each format is its own rating scale.
 */
public final class BotRoster {

    private static final Logger logger = Logger.getLogger(BotRoster.class);
    private static final String FILE = "/ladder/bots.json";

    /** A bot's look: a name and cosmetics from the catalog */
    public static class Persona {
        public String name;
        public String avatar;
        public String sleeve;
        public String playmat;
        public String frame;
        public String title;
    }

    public static class Bot {
        public String id;
        public String format;
        /** a school deck under resources/career/campaigns/decks */
        public String deck;
        public String deckName;
        public String colors;
        /** the school deck's kind: starter, signature, act1, act2, act3, meta, budget (a meta list made cheap, for the League), rogue, tier2 */
        public String kind;
        /** macro archetype for matchmaking diversity and AI profiles: aggro, burn, tempo, midrange, control, ramp, combo, graveyard, sacrifice, tribal */
        public String archetype;
        /** power tier: starter, budget, rogue, tier2, meta */
        public String tier;
        /** optional AI playbook id under playbooks/ (e.g. standard/meta-azorius-control or profile:control) */
        public String playbook;
        /** short playstyle description for the ranked UI */
        public String tagline;
        /** signature cards for scouting and post-match summary */
        public List<String> keyCards;
        /** the queues it plays in (open, league; Bo3 plays the Open queue's); none listed: both */
        public List<String> queues;
        /** the AI skill the Rust host plays it at (1 quick, 3 light, 5 half, 7 full search) */
        public int skill;
        public String search;
        /**
         * The search settings it was rated with, pinned (#435): the Rust host plays this label instead of what a later
         * build makes of the skill, so the bot plays as measured. "quick" is the quick answers alone; null follows the
         * skill. The Java AI, for games the Rust host can't play, still plays the skill.
         */
        public String ai;
        /** the AI generation it was made in (#435): a stronger Rust AI adds bots of a new generation beside the old */
        public int generation;
        /** no longer picked (#435); kept so the games against it still show who it was */
        public boolean retired;
        /** the rating guessed before calibration */
        public int prior;
        public Persona persona;
        /** measured; null until calibrated */
        public Integer rating;
        public Integer rd;
        public Integer games;
        public Integer won;
        public Integer drew;
        public Integer lost;

        /** whether the bot plays in the queue */
        public boolean plays(String queue) {
            if (retired) {
                return false;
            }
            String plays = RankedService.botQueue(queue);
            return queues == null || queues.isEmpty() || queues.contains(plays);
        }

        public boolean calibrated() {
            return rating != null && games != null && games > 0;
        }

        /** the bot's rating as the ladder starts it: measured, or the prior with nothing known */
        public Glicko2.Rating startRating() {
            return calibrated()
                    ? new Glicko2.Rating(rating, rd == null ? Glicko2.MAX_RD : rd, Glicko2.START_VOLATILITY)
                    : new Glicko2.Rating(prior, Glicko2.MAX_RD, Glicko2.START_VOLATILITY);
        }

        /** normalized macro archetype: explicit metadata, or inferred from deck name/path for older entries */
        public String effectiveArchetype() {
            if (archetype != null && !archetype.isEmpty()) {
                return archetype;
            }
            String text = ((deckName == null ? "" : deckName) + " " + (deck == null ? "" : deck)).toLowerCase(java.util.Locale.ENGLISH);
            if (text.contains("burn")) {
                return "burn";
            }
            if (text.contains("control")) {
                return "control";
            }
            if (text.contains("sacrifice") || text.contains("aristocrat")) {
                return "sacrifice";
            }
            if (text.contains("reanimat") || text.contains("goryo") || text.contains("grave") || text.contains("phoenix") || text.contains("greasefang") || text.contains("dredge")) {
                return "graveyard";
            }
            if (text.contains("combo") || text.contains("storm") || text.contains("living-end") || text.contains("rhinos")) {
                return "combo";
            }
            if (text.contains("ramp") || text.contains("landfall") || text.contains("tron") || text.contains("devotion") || text.contains("domain")) {
                return "ramp";
            }
            if (text.contains("tempo") || text.contains("prowess") || text.contains("spirits") || text.contains("skies") || text.contains("merfolk") || text.contains("rogue") || text.contains("spellementals")) {
                return "tempo";
            }
            if (text.contains("aggro") || text.contains("weenie") || text.contains("mono-red") || text.contains("red line") || text.contains("red deck wins") || text.contains("zoo") || text.contains("lizards")) {
                return "aggro";
            }
            if (text.contains("humans") || text.contains("vampires") || text.contains("angels") || text.contains("dragons") || text.contains("elves") || text.contains("goblins")) {
                return "tribal";
            }
            return "midrange";
        }

        /** normalized tier: explicit metadata, or derived from kind */
        public String effectiveTier() {
            if (tier != null && !tier.isEmpty()) {
                return tier;
            }
            if ("meta".equals(kind) || "act3".equals(kind)) {
                return "meta";
            }
            if ("tier2".equals(kind) || "act2".equals(kind)) {
                return "tier2";
            }
            if ("rogue".equals(kind)) {
                return "rogue";
            }
            if ("budget".equals(kind)) {
                return "budget";
            }
            return "starter";
        }
    }

    /** how the file's ratings were measured */
    public static class Calibration {
        public String commit;
        public String harness;
        public Integer turnCap;
        public Integer games;
        public Map<String, String> skills;
    }

    private static class File {
        Map<String, String> anchors;
        Calibration calibration;
        List<Bot> bots;
    }

    private static volatile BotRoster instance;

    private final Map<String, Bot> bots = new LinkedHashMap<>();
    private final Map<String, Persona> personasByName = new LinkedHashMap<>();
    private final Map<String, String> anchors;
    private final Calibration calibration;

    BotRoster(Reader reader) {
        File file = new Gson().fromJson(reader, File.class);
        anchors = file.anchors == null ? Collections.<String, String>emptyMap() : file.anchors;
        calibration = file.calibration;
        for (Bot bot : file.bots) {
            if (bots.put(bot.id, bot) != null) {
                throw new IllegalStateException("Two ladder bots are called " + bot.id);
            }
            if (bot.persona != null && bot.persona.name != null && !bot.persona.name.trim().isEmpty()) {
                personasByName.putIfAbsent(bot.persona.name.trim().toLowerCase(java.util.Locale.ROOT), bot.persona);
            }
        }
    }

    public static BotRoster get() {
        BotRoster current = instance;
        if (current == null) {
            synchronized (BotRoster.class) {
                current = instance;
                if (current == null) {
                    try (InputStream in = BotRoster.class.getResourceAsStream(FILE)) {
                        if (in == null) {
                            throw new IOException(FILE + " is missing");
                        }
                        current = new BotRoster(new InputStreamReader(in, StandardCharsets.UTF_8));
                    } catch (IOException | RuntimeException e) {
                        logger.error("Ladder bots can't be read", e);
                        throw new IllegalStateException("The ladder's bots are missing from this server");
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    public Bot bot(String id) {
        return bots.get(id);
    }

    /** the cosmetic persona for a ladder bot's display name (case-insensitive), or null if none */
    public Persona personaByName(String name) {
        if (name == null) {
            return null;
        }
        String key = name.trim().toLowerCase(java.util.Locale.ROOT);
        return key.isEmpty() ? null : personasByName.get(key);
    }

    public List<Bot> bots() {
        return Collections.unmodifiableList(new ArrayList<>(bots.values()));
    }

    public Set<String> formats() {
        Set<String> out = new LinkedHashSet<>();
        for (Bot bot : bots.values()) {
            out.add(bot.format);
        }
        return out;
    }

    /** a format's bots that play in the queue, weakest first by their starting rating */
    public List<Bot> bots(String format, String queue) {
        List<Bot> out = new ArrayList<>();
        for (Bot bot : bots(format)) {
            if (bot.plays(queue)) {
                out.add(bot);
            }
        }
        return out;
    }

    /** a format's bots, weakest first by their starting rating */
    public List<Bot> bots(String format) {
        List<Bot> out = new ArrayList<>();
        for (Bot bot : bots.values()) {
            if (bot.format.equals(format)) {
                out.add(bot);
            }
        }
        out.sort(Comparator.comparingDouble((Bot b) -> b.startRating().rating).thenComparing(b -> b.id));
        return out;
    }

    /** the format's bot whose rating is 1000 by definition */
    public String anchor(String format) {
        return anchors.get(format);
    }

    public Calibration calibration() {
        return calibration;
    }

    /** the classpath resource path for a bot's deck file */
    public static String deckResource(String deckPath) {
        if (deckPath != null && deckPath.startsWith("ladder/")) {
            return "/ladder/decks/" + deckPath.substring("ladder/".length());
        }
        return "/career/campaigns/decks/" + deckPath;
    }

    /** the bot's deck, as the server plays it */
    public DeckCardLists deck(Bot bot) throws IOException {
        if (bot.deck != null && bot.deck.startsWith("ladder/")) {
            String resource = deckResource(bot.deck);
            InputStream stream = BotRoster.class.getResourceAsStream(resource);
            if (stream == null) {
                throw new IOException("Missing ladder deck " + resource);
            }
            java.nio.file.Path file = java.nio.file.Files.createTempFile("ladder-", ".dck");
            try {
                try (InputStream in = stream) {
                    java.nio.file.Files.copy(in, file, java.nio.file.StandardCopyOption.REPLACE_EXISTING);
                }
                StringBuilder errors = new StringBuilder();
                DeckCardLists deck = mage.cards.decks.importer.DeckImporter.importDeckFromFile(file.toString(), errors, false);
                if (deck == null || deck.getCards().isEmpty()) {
                    throw new IOException("Ladder deck " + resource + " can't be read: " + errors);
                }
                return deck;
            } finally {
                java.nio.file.Files.deleteIfExists(file);
            }
        }
        return CareerCampaigns.get().deck(bot.deck);
    }
}
