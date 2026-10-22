package mage.server.career;

import com.google.gson.Gson;
import mage.cards.decks.DeckCardLists;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The meta library: the school campaigns' decks by format, each with the act it's fought in as its tier (S032, S045).
 * The decks carry the snapshot date of resources/career/meta.json; a school that's rebuilt moves the date.
 */
public final class MetaLibrary {

    /** One deck of the library. */
    public static class MetaDeck {
        /** the deck's file under campaigns/decks, its id */
        public String file;
        public String name;
        /** the school's format id: "modern" */
        public String format;
        public String formatName;
        /** the act it's met in (1 to 4): a rough tier, higher is stronger */
        public int tier;
        /** who pilots it in the school */
        public String pilot;
        public boolean boss;
        public String deckType;
        public String gameType;
        public String colors;
    }

    /** A format of the library, for the free-play picker. */
    public static class MetaFormat {
        public String id;
        public String name;
        public String snapshot;
        public List<MetaDeck> decks = new ArrayList<>();
    }

    static class Meta {
        String snapshot;
        String text;
        List<String> formats;
    }

    private static volatile MetaLibrary instance;

    private final Meta meta;
    private final Map<String, MetaFormat> formats = new LinkedHashMap<>();

    private MetaLibrary() throws IOException {
        InputStream stream = MetaLibrary.class.getResourceAsStream("/career/meta.json");
        if (stream == null) {
            throw new IOException("Missing career resource meta.json");
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            meta = new Gson().fromJson(reader, Meta.class);
        }
        Map<String, CareerCampaigns.Campaign> campaigns = CareerCampaigns.get().campaigns();
        for (String id : meta.formats) {
            CareerCampaigns.Campaign school = campaigns.get(id);
            if (school == null || school.chapters == null) {
                continue;
            }
            MetaFormat format = new MetaFormat();
            format.id = id;
            format.name = school.format == null ? school.name : school.format;
            format.snapshot = meta.snapshot;
            Map<String, MetaDeck> decks = new LinkedHashMap<>();
            for (int act = 0; act < school.chapters.size(); act++) {
                CareerCampaigns.Chapter chapter = school.chapters.get(act);
                for (CareerCampaigns.Node node : chapter.nodes == null ? Collections.<CareerCampaigns.Node>emptyList() : chapter.nodes) {
                    if (node.opponent == null || node.opponent.deck == null || decks.containsKey(node.opponent.deck)) {
                        continue;
                    }
                    MetaDeck deck = new MetaDeck();
                    deck.file = node.opponent.deck;
                    deck.format = id;
                    deck.formatName = format.name;
                    deck.tier = act + 1;
                    deck.pilot = node.opponent.name;
                    deck.boss = node.boss;
                    deck.deckType = node.deckType != null ? node.deckType : school.deckType;
                    deck.gameType = node.gameType != null ? node.gameType : school.gameType;
                    describe(deck);
                    decks.put(deck.file, deck);
                }
            }
            format.decks.addAll(decks.values());
            formats.put(id, format);
        }
    }

    /** the deck's name and colours, read from its list */
    private static void describe(MetaDeck meta) {
        String base = meta.file.substring(meta.file.lastIndexOf('/') + 1).replace(".dck", "");
        meta.name = base.replaceFirst("^act\\d-", "").replace('-', ' ');
        try {
            DeckCardLists deck = CareerCampaigns.get().deck(meta.file);
            if (deck.getName() != null && !deck.getName().isEmpty()) {
                meta.name = deck.getName();
            }
            StringBuilder colors = new StringBuilder();
            for (mage.cards.decks.DeckCardInfo card : deck.getCards()) {
                mage.cards.repository.CardInfo info = mage.cards.repository.CardRepository.instance.findPreferredCoreExpansionCard(card.getCardName());
                if (info == null) {
                    continue;
                }
                mage.ObjectColor color = info.getColor();
                for (char letter : "WUBRG".toCharArray()) {
                    boolean has = letter == 'W' ? color.isWhite() : letter == 'U' ? color.isBlue() : letter == 'B' ? color.isBlack()
                            : letter == 'R' ? color.isRed() : color.isGreen();
                    if (has && colors.indexOf(String.valueOf(letter)) < 0) {
                        colors.append(letter);
                    }
                }
            }
            meta.colors = "WUBRG".chars().filter(c -> colors.indexOf(String.valueOf((char) c)) >= 0)
                    .collect(StringBuilder::new, StringBuilder::appendCodePoint, StringBuilder::append).toString();
        } catch (IOException | RuntimeException e) {
            // the file name will do
        }
    }

    public static MetaLibrary get() {
        MetaLibrary current = instance;
        if (current == null) {
            synchronized (MetaLibrary.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new MetaLibrary();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("The meta library can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    public String snapshot() {
        return meta.snapshot;
    }

    public List<MetaFormat> formats() {
        return new ArrayList<>(formats.values());
    }

    public MetaFormat format(String id) {
        return formats.get(id);
    }

    /** every deck of the 60-card formats (two-player constructed, not Commander or Brawl) */
    public List<MetaDeck> constructed() {
        List<MetaDeck> decks = new ArrayList<>();
        for (MetaFormat format : formats.values()) {
            for (MetaDeck deck : format.decks) {
                if (deck.deckType != null && deck.deckType.startsWith("Constructed")) {
                    decks.add(deck);
                }
            }
        }
        return decks;
    }

    public MetaDeck deck(String file) {
        for (MetaFormat format : formats.values()) {
            for (MetaDeck deck : format.decks) {
                if (deck.file.equals(file)) {
                    return deck;
                }
            }
        }
        return null;
    }
}
