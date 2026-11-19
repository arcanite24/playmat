package mage.server.websocket.api;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.server.career.CareerStore;
import mage.server.solo.SoloStore;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import org.apache.log4j.Logger;

import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Home (#432): the widget board's layout, kept per account so it follows the player between devices, and a short
 * summary of the games played lately for the Recent and Streak widgets.
 */
public final class HomeApi {

    private static final Logger logger = Logger.getLogger(HomeApi.class);
    static final String LAYOUT_KEY = "home.layout";
    /** a layout is a short list; anything longer than this isn't one */
    static final int MAX_JSON = 8192;
    static final int MAX_TEXT = 40;
    static final int MAX_SETTINGS = 4;
    static final Set<String> PRESETS = new HashSet<>(Arrays.asList("competitor", "collector", "casual", "minimal"));
    /** the days the Streak widget shows */
    static final int DAYS = 14;
    static final int RECENT = 8;
    static final long DAY_MS = 24L * 60 * 60 * 1000;

    private static volatile Widgets widgets;

    private HomeApi() {
    }

    /** The widgets a layout may hold, from home/widgets.json */
    static final class Widgets {
        final int max;
        final Map<String, Set<String>> sizes = new LinkedHashMap<>();

        Widgets(JsonObject json) {
            max = json.get("max").getAsInt();
            for (Map.Entry<String, JsonElement> entry : json.getAsJsonObject("widgets").entrySet()) {
                Set<String> allowed = new HashSet<>();
                for (JsonElement size : entry.getValue().getAsJsonArray()) {
                    allowed.add(size.getAsString());
                }
                sizes.put(entry.getKey(), allowed);
            }
        }
    }

    static Widgets widgets() {
        Widgets current = widgets;
        if (current == null) {
            try (InputStream in = HomeApi.class.getResourceAsStream("/home/widgets.json")) {
                current = new Widgets(JsonParser.parseReader(new InputStreamReader(in, StandardCharsets.UTF_8)).getAsJsonObject());
            } catch (Exception e) {
                throw new IllegalStateException("home/widgets.json can't be read", e);
            }
            widgets = current;
        }
        return current;
    }

    /** One game for the Recent widget */
    public static class HomeGame {
        public long at;
        public Boolean won;
        public String opponent;
        public boolean career;
    }

    /** What the account played lately */
    public static class HomeActivity {
        /** games per day, oldest first, the last one today (in the player's time zone) */
        public List<Integer> days;
        /** days in a row with a game, up to today (or yesterday, when today has none yet) */
        public int streak;
        public List<HomeGame> games;
    }

    public static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("homeLayoutGet")
                        .returns("string | null")
                        .doc("The signed-in account's Home widget layout as JSON ({ v: 1, preset, items: [{ key, id, size, settings }] }), "
                                + "or null before it saves one.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            return solo(() -> SoloStore.get().state(user, LAYOUT_KEY));
                        }),

                RpcMethod.named("homeLayoutSet")
                        .params(of("layout", STRING))
                        .returns("string")
                        .doc("Saves the account's Home widget layout. Unknown widgets and sizes are refused; at most 24 widgets. "
                                + "Returns the layout as saved.")
                        .handler(call -> {
                            String user = CareerApi.user(ctx, call);
                            String clean = clean(call.string(0));
                            solo(() -> {
                                SoloStore.get().putState(user, LAYOUT_KEY, clean);
                                return null;
                            });
                            return clean;
                        }),

                RpcMethod.named("homeActivity")
                        .params(optional("offsetMinutes", INT))
                        .returns("HomeActivity")
                        .doc("The games the account played lately, from its Career record: games per day over two weeks, the "
                                + "streak of days played and the last few results. offsetMinutes is the player's time zone "
                                + "(minutes east of UTC) so days end at their midnight. Empty without a Career profile.")
                        .handler(call -> {
                            String user = CareerApi.career(ctx, call);
                            int offset = call.has(0) ? call.integer(0) : 0;
                            if (Math.abs(offset) > 14 * 60) {
                                throw RpcException.invalidParams("offsetMinutes is out of range");
                            }
                            return CareerApi.store(() -> activity(user, offset, System.currentTimeMillis()));
                        })
        );
    }

    interface SoloCall<T> {
        T run() throws SQLException;
    }

    static <T> T solo(SoloCall<T> work) throws RpcException {
        try {
            return work.run();
        } catch (SQLException | IllegalStateException e) {
            logger.error("Home layout store failed", e);
            throw new RpcException(RpcException.SERVER_ERROR, "The Home layout can't be read or saved right now", e);
        }
    }

    /** a layout checked against the widget list and written back without anything else */
    static String clean(String json) throws RpcException {
        if (json == null || json.length() > MAX_JSON) {
            throw RpcException.invalidParams("That isn't a Home layout.");
        }
        JsonObject in;
        try {
            in = JsonParser.parseString(json).getAsJsonObject();
        } catch (RuntimeException e) {
            throw RpcException.invalidParams("That isn't a Home layout.");
        }
        if (!in.has("v") || !in.get("v").isJsonPrimitive() || in.get("v").getAsInt() != 1 || !in.has("items") || !in.get("items").isJsonArray()) {
            throw RpcException.invalidParams("That isn't a Home layout.");
        }
        Widgets known = widgets();
        JsonArray items = in.getAsJsonArray("items");
        if (items.size() > known.max) {
            throw RpcException.invalidParams("Home holds at most " + known.max + " widgets.");
        }
        JsonObject out = new JsonObject();
        out.addProperty("v", 1);
        String preset = text(in.get("preset"));
        if (preset != null && PRESETS.contains(preset)) {
            out.addProperty("preset", preset);
        } else {
            out.add("preset", com.google.gson.JsonNull.INSTANCE);
        }
        JsonArray cleanItems = new JsonArray();
        Set<String> keys = new HashSet<>();
        for (JsonElement element : items) {
            if (!element.isJsonObject()) {
                throw RpcException.invalidParams("That isn't a Home layout.");
            }
            JsonObject item = element.getAsJsonObject();
            String id = text(item.get("id"));
            String size = text(item.get("size"));
            String key = text(item.get("key"));
            if (id == null || !known.sizes.containsKey(id)) {
                throw RpcException.invalidParams("There is no Home widget " + id + ".");
            }
            if (size == null || !known.sizes.get(id).contains(size)) {
                throw RpcException.invalidParams("The " + id + " widget doesn't come in size " + size + ".");
            }
            if (key == null || !keys.add(key)) {
                throw RpcException.invalidParams("Every widget needs its own key.");
            }
            JsonObject clean = new JsonObject();
            clean.addProperty("key", key);
            clean.addProperty("id", id);
            clean.addProperty("size", size);
            if (item.has("settings") && item.get("settings").isJsonObject()) {
                JsonObject settings = item.getAsJsonObject("settings");
                if (settings.size() > MAX_SETTINGS) {
                    throw RpcException.invalidParams("Too many settings on the " + id + " widget.");
                }
                JsonObject cleanSettings = new JsonObject();
                for (Map.Entry<String, JsonElement> setting : settings.entrySet()) {
                    String value = text(setting.getValue());
                    if (setting.getKey().length() > MAX_TEXT || value == null) {
                        throw RpcException.invalidParams("A setting on the " + id + " widget isn't valid.");
                    }
                    cleanSettings.addProperty(setting.getKey(), value);
                }
                if (cleanSettings.size() > 0) {
                    clean.add("settings", cleanSettings);
                }
            }
            cleanItems.add(clean);
        }
        out.add("items", cleanItems);
        return new com.google.gson.GsonBuilder().serializeNulls().create().toJson(out);
    }

    /** a short string, or null */
    private static String text(JsonElement element) {
        if (element == null || !element.isJsonPrimitive() || !element.getAsJsonPrimitive().isString()) {
            return null;
        }
        String value = element.getAsString();
        return value.isEmpty() || value.length() > MAX_TEXT ? null : value;
    }

    static HomeActivity activity(String user, int offsetMinutes, long now) throws SQLException {
        long offset = offsetMinutes * 60_000L;
        long today = Math.floorDiv(now + offset, DAY_MS);
        long since = (today - DAYS + 1) * DAY_MS - offset;
        List<CareerStore.PlayedGame> played = CareerStore.get().playedSince(user, since, 2000);
        int[] days = new int[DAYS];
        HomeActivity out = new HomeActivity();
        out.games = new ArrayList<>();
        for (CareerStore.PlayedGame game : played) {
            int index = (int) (Math.floorDiv(game.at + offset, DAY_MS) - (today - DAYS + 1));
            if (index >= 0 && index < DAYS) {
                days[index]++;
            }
            if (out.games.size() < RECENT) {
                out.games.add(game(game));
            }
        }
        out.days = new ArrayList<>();
        for (int count : days) {
            out.days.add(count);
        }
        // today without a game yet doesn't break the streak
        int i = days[DAYS - 1] > 0 ? DAYS - 1 : DAYS - 2;
        while (i >= 0 && days[i] > 0) {
            out.streak++;
            i--;
        }
        return out;
    }

    private static HomeGame game(CareerStore.PlayedGame played) {
        HomeGame game = new HomeGame();
        game.at = played.at;
        if (played.details != null) {
            try {
                JsonObject details = JsonParser.parseString(played.details).getAsJsonObject();
                if (details.has("won") && details.get("won").isJsonPrimitive()) {
                    game.won = details.get("won").getAsBoolean();
                }
                if (details.has("opponent") && details.get("opponent").isJsonPrimitive()) {
                    game.opponent = details.get("opponent").getAsString();
                }
                game.career = details.has("career") && details.get("career").isJsonPrimitive() && details.get("career").getAsBoolean();
            } catch (RuntimeException e) {
                // an older row without a summary
            }
        }
        return game;
    }
}
