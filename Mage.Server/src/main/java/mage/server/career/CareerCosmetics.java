package mage.server.career;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.reflect.TypeToken;

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
 * Every cosmetic progression can give, in one catalog (resources/career/cosmetics.json, B293): avatars, titles, plate
 * frames, sleeves, playmats, companions, emote sets, attack arrows, hit sparks, victory stamps and basic land styles,
 * each with its name (English and Spanish), rarity and what the client needs to draw it. Where an item comes from is
 * read from the content that gives it (levels, achievements, campaigns) or, for masteries, from the item itself; a
 * free item (the table-talk phrases) comes from nowhere. The supporters' thank-you set is not here: it is bought, not
 * earned (support/SupportPerks).
 */
public final class CareerCosmetics {

    public static final List<String> KINDS = Collections.unmodifiableList(java.util.Arrays.asList(
            "avatar", "title", "frame", "sleeve", "playmat", "pet", "emote", "arrow", "spark", "stamp", "land", "cardstyle"));

    /** One sticker or phrase of an emote set. */
    public static class CareerSticker {
        public String id;
        public String phrase;
        public String es;
    }

    /** Where a cosmetic comes from. */
    public static class CareerCosmeticSource {
        /** "level", "achievement", "campaign", "mastery", "free", "capsule" (pulled from a machine, or finishing it) or "capsuleSet" (finishing a capsule set) */
        public String kind;
        /** the achievement, campaign or mastery track; null for a level */
        public String ref;
        /** the level, the achievement tier (1 to 4, 0 for an achievement without tiers) or the mastery rank */
        public int at;

        public CareerCosmeticSource() {
        }

        CareerCosmeticSource(String kind, String ref, int at) {
            this.kind = kind;
            this.ref = ref;
            this.at = at;
        }
    }

    /** A catalog item, with how it looks. */
    public static class CareerCatalogItem {
        public String kind;
        public String id;
        public String name;
        /** the Spanish name */
        public String es;
        /** common, uncommon, rare or mythic */
        public String rarity;
        /** a sleeve's dye, an arrow's or a spark's colour (#rrggbb) */
        public String color;
        /** an avatar medallion's colours and its line drawing (24px box) for when painted art is off */
        public String light;
        public String dark;
        public String path;
        /** a playmat cloth's dye and seven shades, darkest first */
        public String dye;
        public List<String> shades;
        /** a plate frame's ring and accent colours, and its ornament: solid, double, dashed, beaded, notched or rays */
        public String ring;
        public String accent;
        public String style;
        /** a basic land style's set code */
        public String set;
        /** a card style's card (its English name) and, for card styles and a capsule set's sleeve, the capsule set */
        public String card;
        public String series;
        /** an emote set's stickers (or phrases) */
        public List<CareerSticker> stickers;
        /** a mastery track and rank that give it (else the content says) */
        public String mastery;
        public int rank;
        /** everyone has it */
        public boolean free;
        /** every place that gives it */
        public List<CareerCosmeticSource> sources = new ArrayList<>();
    }

    private static volatile CareerCosmetics instance;

    private final List<CareerCatalogItem> items;
    private final Map<String, CareerCatalogItem> byKey = new LinkedHashMap<>();

    CareerCosmetics(List<CareerCatalogItem> items) {
        this.items = items;
        for (CareerCatalogItem item : items) {
            byKey.put(key(item.kind, item.id), item);
            if (item.mastery != null) {
                item.sources.add(new CareerCosmeticSource("mastery", item.mastery, item.rank));
            }
            if (item.free) {
                item.sources.add(new CareerCosmeticSource("free", null, 0));
            }
        }
    }

    public static CareerCosmetics get() {
        CareerCosmetics current = instance;
        if (current == null) {
            synchronized (CareerCosmetics.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = load();
                    } catch (IOException e) {
                        throw new IllegalStateException("Career cosmetics can't be read", e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    private static CareerCosmetics load() throws IOException {
        List<CareerCatalogItem> items = new Gson().fromJson(text("cosmetics.json"), new TypeToken<List<CareerCatalogItem>>() {
        }.getType());
        CareerCapsules capsules = CareerCapsules.get();
        items.addAll(capsules.catalogItems());
        CareerCosmetics catalog = new CareerCosmetics(items);
        for (CareerCapsules.CareerMachine machine : capsules.machines()) {
            catalog.source(machine.reward, new CareerCosmeticSource("capsule", machine.id, 0));
            catalog.source(machine.title, new CareerCosmeticSource("capsule", machine.id, 0));
        }
        CareerProgress progress = CareerProgress.get();
        for (CareerProgress.CareerLevelReward level : progress.levelRewards().values()) {
            catalog.source(level.cosmetic, new CareerCosmeticSource("level", null, level.level));
        }
        for (CareerProgress.AchievementTemplate achievement : progress.achievementTemplates().values()) {
            catalog.source(achievement.cosmetic, new CareerCosmeticSource("achievement", achievement.id, 0));
            if (achievement.tiers != null) {
                for (int tier = 0; tier < achievement.tiers.size(); tier++) {
                    catalog.source(achievement.tiers.get(tier).cosmetic, new CareerCosmeticSource("achievement", achievement.id, tier + 1));
                }
            }
        }
        try {
            for (String campaign : CareerCampaigns.get().campaigns().keySet()) {
                String json = text("campaigns/" + campaign + ".json");
                for (CareerProgress.CareerCosmetic cosmetic : cosmeticsIn(JsonParser.parseString(json))) {
                    catalog.source(cosmetic, new CareerCosmeticSource("campaign", campaign, 0));
                }
            }
        } catch (RuntimeException e) {
            // no campaigns on this server
        }
        return catalog;
    }

    private static String text(String name) throws IOException {
        InputStream stream = CareerCosmetics.class.getResourceAsStream("/career/" + name);
        if (stream == null) {
            throw new IOException("Missing career resource " + name);
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            StringBuilder text = new StringBuilder();
            char[] buffer = new char[8192];
            for (int read; (read = reader.read(buffer)) > 0; ) {
                text.append(buffer, 0, read);
            }
            return text.toString();
        }
    }

    /** every {kind, id} under a "cosmetic" or in a "cosmetics" list, anywhere in a content file */
    static List<CareerProgress.CareerCosmetic> cosmeticsIn(JsonElement element) {
        List<CareerProgress.CareerCosmetic> found = new ArrayList<>();
        collect(element, found);
        return found;
    }

    private static void collect(JsonElement element, List<CareerProgress.CareerCosmetic> found) {
        if (element.isJsonArray()) {
            for (JsonElement child : element.getAsJsonArray()) {
                collect(child, found);
            }
        } else if (element.isJsonObject()) {
            JsonObject object = element.getAsJsonObject();
            JsonElement one = object.get("cosmetic");
            if (one != null && one.isJsonObject()) {
                add(one.getAsJsonObject(), found);
            }
            JsonElement many = object.get("cosmetics");
            if (many != null && many.isJsonArray()) {
                JsonArray list = many.getAsJsonArray();
                for (JsonElement cosmetic : list) {
                    if (cosmetic.isJsonObject()) {
                        add(cosmetic.getAsJsonObject(), found);
                    }
                }
            }
            for (Map.Entry<String, JsonElement> entry : object.entrySet()) {
                if (!"cosmetic".equals(entry.getKey()) && !"cosmetics".equals(entry.getKey())) {
                    collect(entry.getValue(), found);
                }
            }
        }
    }

    private static void add(JsonObject cosmetic, List<CareerProgress.CareerCosmetic> found) {
        if (cosmetic.has("kind") && cosmetic.has("id")) {
            found.add(new CareerProgress.CareerCosmetic(cosmetic.get("kind").getAsString(), cosmetic.get("id").getAsString()));
        }
    }

    private void source(CareerProgress.CareerCosmetic cosmetic, CareerCosmeticSource source) {
        if (cosmetic == null) {
            return;
        }
        CareerCatalogItem item = byKey.get(key(cosmetic.kind, cosmetic.id));
        if (item != null) {
            item.sources.add(source);
        }
    }

    private static String key(String kind, String id) {
        return kind + ':' + id;
    }

    public List<CareerCatalogItem> items() {
        return Collections.unmodifiableList(items);
    }

    public CareerCatalogItem find(String kind, String id) {
        return kind == null || id == null ? null : byKey.get(key(kind, id));
    }

    /** the cosmetic a mastery rank gives, or null */
    public CareerProgress.CareerCosmetic masteryReward(String track, int rank) {
        for (CareerCatalogItem item : items) {
            if (track.equals(item.mastery) && item.rank == rank) {
                return new CareerProgress.CareerCosmetic(item.kind, item.id);
            }
        }
        return null;
    }

    /** whether everyone has this cosmetic */
    public boolean isFree(String kind, String id) {
        CareerCatalogItem item = find(kind, id);
        return item != null && item.free;
    }

    /** the emote set a sticker or phrase belongs to, or null */
    public CareerCatalogItem emoteSetOf(String sticker) {
        for (CareerCatalogItem item : items) {
            if ("emote".equals(item.kind) && item.stickers != null) {
                for (CareerSticker one : item.stickers) {
                    if (one.id.equals(sticker)) {
                        return item;
                    }
                }
            }
        }
        return null;
    }
}
