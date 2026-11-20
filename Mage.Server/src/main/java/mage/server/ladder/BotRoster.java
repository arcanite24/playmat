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
        /** the school deck's kind: starter, signature, act1, act2, act3, meta, budget (a meta list made cheap, for the League) */
        public String kind;
        /** the queues it plays in (open, league); none listed: both */
        public List<String> queues;
        /** the AI skill the Rust host plays it at (1 quick, 3 light, 5 half, 7 full search) */
        public int skill;
        public String search;
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
            return queues == null || queues.isEmpty() || queues.contains(queue);
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

    /** the bot's deck, as the server plays it */
    public DeckCardLists deck(Bot bot) throws IOException {
        return CareerCampaigns.get().deck(bot.deck);
    }
}
