package mage.server.websocket.service;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import mage.cards.decks.Deck;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.constants.MatchBufferTime;
import mage.constants.MatchTimeLimit;
import mage.constants.MultiplayerAttackOption;
import mage.constants.RangeOfInfluence;
import mage.constants.SkillLevel;
import mage.game.draft.DraftOptions;
import mage.game.match.MatchOptions;
import mage.game.mulligan.MulliganType;
import mage.game.tournament.LimitedOptions;
import mage.game.tournament.TournamentOptions;
import mage.players.PlayerType;
import mage.server.websocket.rpc.JsonCodec;
import mage.server.websocket.rpc.RpcException;
import org.apache.log4j.Logger;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;

/**
 * Web client bridge: maps the web client's table/tournament option objects to server option classes.
 * Unknown keys are ignored; malformed known values are reported as invalid params.
 */
public final class OptionsMapper {

    private static final Logger logger = Logger.getLogger(OptionsMapper.class);

    private OptionsMapper() {
    }

    public static MatchOptions toMatchOptions(JsonObject json) throws RpcException {
        String tableName = str(json, "name", "Game");
        String gameType = str(json, "gameType", "Two Player Duel");
        MatchOptions matchOptions = new MatchOptions(tableName, gameType, isMultiplayerGameType(gameType));
        applyMatchOptions(json, matchOptions);
        return matchOptions;
    }

    public static TournamentOptions toTournamentOptions(JsonObject json) throws RpcException {
        String name = str(json, "name", "Tournament");
        String tournamentType = str(json, "tournamentType", "Constructed");
        JsonObject matchJson = json.has("matchOptions") && json.get("matchOptions").isJsonObject()
                ? json.get("matchOptions").getAsJsonObject()
                : new JsonObject();
        String gameType = str(matchJson, "gameType", "Two Player Duel");
        boolean multiPlayer = json.has("singleMultiplayerGame")
                ? bool(json, "singleMultiplayerGame")
                : isMultiplayerGameType(gameType);

        TournamentOptions options = new TournamentOptions(name, gameType, multiPlayer);
        options.setTournamentType(tournamentType);
        options.setPassword(str(json, "password", ""));
        applyMatchOptions(matchJson, options.getMatchOptions());

        if (json.has("playerTypes") && json.get("playerTypes").isJsonArray()) {
            for (JsonElement playerType : json.get("playerTypes").getAsJsonArray()) {
                options.getPlayerTypes().add(toPlayerType(playerType));
            }
        }
        if (json.has("watchingAllowed")) options.setWatchingAllowed(bool(json, "watchingAllowed"));
        if (json.has("planeChase")) options.setPlaneChase(bool(json, "planeChase"));
        if (json.has("numberRounds")) options.setNumberRounds(integer(json, "numberRounds"));
        options.setQuitRatio(json.has("quitRatio") ? integer(json, "quitRatio") : 100);
        if (json.has("minimumRating")) options.setMinimumRating(integer(json, "minimumRating"));

        JsonObject limitedJson = json.has("limitedOptions") && json.get("limitedOptions").isJsonObject()
                ? json.get("limitedOptions").getAsJsonObject()
                : new JsonObject();
        // like the desktop client: tournaments always carry limited options, the server relies on it
        options.setLimitedOptions(toLimitedOptions(limitedJson, tournamentType));
        return options;
    }

    private static void applyMatchOptions(JsonObject json, MatchOptions options) throws RpcException {
        if (json.has("deckType")) options.setDeckType(str(json, "deckType", null));
        if (json.has("winsNeeded")) options.setWinsNeeded(integer(json, "winsNeeded"));
        if (json.has("freeMulligans")) options.setFreeMulligans(integer(json, "freeMulligans"));
        if (json.has("customStartLifeEnabled")) options.setCustomStartLifeEnabled(bool(json, "customStartLifeEnabled"));
        if (json.has("customStartLife")) options.setCustomStartLife(integer(json, "customStartLife"));
        if (json.has("customStartHandSizeEnabled")) options.setCustomStartHandSizeEnabled(bool(json, "customStartHandSizeEnabled"));
        if (json.has("customStartHandSize")) options.setCustomStartHandSize(integer(json, "customStartHandSize"));
        if (json.has("planeChase")) options.setPlaneChase(bool(json, "planeChase"));
        if (hasValue(json, "password")) options.setPassword(str(json, "password", ""));
        if (json.has("limited")) options.setLimited(bool(json, "limited"));
        if (json.has("rated")) options.setRated(bool(json, "rated"));
        if (json.has("rollbackTurnsAllowed")) options.setRollbackTurnsAllowed(bool(json, "rollbackTurnsAllowed"));
        if (json.has("spectatorsAllowed")) options.setSpectatorsAllowed(bool(json, "spectatorsAllowed"));
        // a seeded match deals the same cards every time it's played with the same seed
        if (hasValue(json, "seed")) options.setSeed(json.get("seed").getAsLong());
        if (hasValue(json, "matchTimeLimit")) options.setMatchTimeLimit(enumValue(json, "matchTimeLimit", MatchTimeLimit.class));
        if (hasValue(json, "matchBufferTime")) options.setMatchBufferTime(enumValue(json, "matchBufferTime", MatchBufferTime.class));
        if (hasValue(json, "attackOption")) options.setAttackOption(enumValue(json, "attackOption", MultiplayerAttackOption.class));
        if (hasValue(json, "range")) options.setRange(enumValue(json, "range", RangeOfInfluence.class));
        if (hasValue(json, "skillLevel")) options.setSkillLevel(enumValue(json, "skillLevel", SkillLevel.class));
        if (hasValue(json, "mulliganType")) options.setMullgianType(enumValue(json, "mulliganType", MulliganType.class));
        if (json.has("perPlayerEmblemCards")) options.setPerPlayerEmblemCards(toDeckCardInfos(json.get("perPlayerEmblemCards")));
        if (json.has("globalEmblemCards")) options.setGlobalEmblemCards(toDeckCardInfos(json.get("globalEmblemCards")));

        // web tables allow everyone by default instead of blocking players with any quit history
        options.setQuitRatio(json.has("quitRatio") ? integer(json, "quitRatio") : 100);
        if (json.has("minimumRating")) options.setMinimumRating(integer(json, "minimumRating"));
        if (json.has("edhPowerLevel")) options.setEdhPowerLevel(integer(json, "edhPowerLevel"));
        if (json.has("bannedUsers") && json.get("bannedUsers").isJsonArray()) {
            Set<String> bannedUsers = new HashSet<>();
            for (JsonElement user : json.get("bannedUsers").getAsJsonArray()) {
                if (!user.isJsonNull()) {
                    bannedUsers.add(user.getAsString());
                }
            }
            options.setBannedUsers(bannedUsers);
        }
        if (json.has("playerTypes") && json.get("playerTypes").isJsonArray()) {
            for (JsonElement playerType : json.get("playerTypes").getAsJsonArray()) {
                options.getPlayerTypes().add(toPlayerType(playerType));
            }
        }
        if (options.getPlayerTypes().isEmpty()) {
            options.getPlayerTypes().add(PlayerType.HUMAN);
            options.getPlayerTypes().add(PlayerType.HUMAN);
        }
    }

    private static LimitedOptions toLimitedOptions(JsonObject json, String tournamentType) throws RpcException {
        LimitedOptions options = tournamentType.toLowerCase(Locale.ENGLISH).contains("draft")
                ? new DraftOptions()
                : new LimitedOptions();

        if (json.has("sets") && json.get("sets").isJsonArray()) {
            for (JsonElement set : json.get("sets").getAsJsonArray()) {
                options.getSetCodes().add(set.getAsString());
            }
        }
        if (json.has("constructionTime")) options.setConstructionTime(integer(json, "constructionTime"));
        if (hasValue(json, "draftCubeName")) options.setDraftCubeName(str(json, "draftCubeName", null));
        if (json.has("cubeFromDeck") && json.get("cubeFromDeck").isJsonObject()) {
            try {
                Deck cube = Deck.load(JsonCodec.GSON.fromJson(json.get("cubeFromDeck"), DeckCardLists.class), true, true);
                cube.clearLayouts();
                options.setCubeFromDeck(cube);
            } catch (Exception e) {
                logger.warn("Can't load cube from deck for a web tournament: " + e.getMessage());
                throw RpcException.invalidParams("Cube deck could not be loaded: " + e.getMessage());
            }
        }
        if (hasValue(json, "jumpstartPacks")) options.setJumpstartPacks(str(json, "jumpstartPacks", null));
        if (json.has("numberBoosters")) options.setNumberBoosters(integer(json, "numberBoosters"));
        if (json.has("isRandom")) options.setIsRandom(bool(json, "isRandom"));
        if (json.has("isReshuffled")) options.setIsReshuffled(bool(json, "isReshuffled"));
        if (json.has("isRichMan")) options.setIsRichMan(bool(json, "isRichMan"));
        if (json.has("isJumpstart")) options.setIsJumpstart(bool(json, "isJumpstart"));

        if (options instanceof DraftOptions) {
            DraftOptions.TimingOption timing = hasValue(json, "timing")
                    ? enumValue(json, "timing", DraftOptions.TimingOption.class)
                    : DraftOptions.TimingOption.REGULAR;
            ((DraftOptions) options).setTiming(timing);
        }
        return options;
    }

    private static List<DeckCardInfo> toDeckCardInfos(JsonElement cardsJson) {
        List<DeckCardInfo> cards = new ArrayList<>();
        if (cardsJson == null || !cardsJson.isJsonArray()) {
            return cards;
        }
        for (JsonElement cardJson : cardsJson.getAsJsonArray()) {
            if (cardJson == null || !cardJson.isJsonObject()) {
                continue;
            }
            JsonObject card = cardJson.getAsJsonObject();
            String cardName = trimmed(card, "cardName");
            String cardNumber = trimmed(card, "cardNumber");
            String setCode = trimmed(card, "setCode");
            if (cardName == null || cardNumber == null || setCode == null) {
                continue;
            }
            int amount = 1;
            if (hasValue(card, "amount")) {
                try {
                    amount = Math.max(1, Math.min(99, card.get("amount").getAsInt()));
                } catch (RuntimeException ignore) {
                    amount = 1;
                }
            }
            cards.add(new DeckCardInfo(cardName, cardNumber, setCode, amount));
        }
        return cards;
    }

    public static PlayerType toPlayerType(JsonElement json) throws RpcException {
        String name = json.getAsString();
        try {
            return PlayerType.valueOf(name);
        } catch (IllegalArgumentException ignored) {
            try {
                return PlayerType.getByDescription(name);
            } catch (IllegalArgumentException e) {
                throw RpcException.invalidParams("Unknown player type: " + name);
            }
        }
    }

    public static PlayerType toPlayerType(String name) throws RpcException {
        return toPlayerType(new com.google.gson.JsonPrimitive(name));
    }

    private static boolean isMultiplayerGameType(String gameType) {
        return gameType.contains("Free For All") || gameType.contains("Commander");
    }

    private static boolean hasValue(JsonObject json, String key) {
        return json.has(key) && !json.get(key).isJsonNull();
    }

    private static String trimmed(JsonObject json, String key) {
        if (!hasValue(json, key)) {
            return null;
        }
        String value = json.get(key).getAsString().trim();
        return value.isEmpty() ? null : value;
    }

    private static String str(JsonObject json, String key, String defaultValue) {
        return hasValue(json, key) ? json.get(key).getAsString() : defaultValue;
    }

    private static int integer(JsonObject json, String key) throws RpcException {
        try {
            return json.get(key).getAsInt();
        } catch (RuntimeException e) {
            throw RpcException.invalidParams("Option '" + key + "' must be an integer");
        }
    }

    private static boolean bool(JsonObject json, String key) throws RpcException {
        JsonElement value = json.get(key);
        if (value == null || !value.isJsonPrimitive() || !value.getAsJsonPrimitive().isBoolean()) {
            throw RpcException.invalidParams("Option '" + key + "' must be a boolean");
        }
        return value.getAsBoolean();
    }

    private static <E extends Enum<E>> E enumValue(JsonObject json, String key, Class<E> type) throws RpcException {
        String value = json.get(key).getAsString();
        try {
            return Enum.valueOf(type, value);
        } catch (IllegalArgumentException e) {
            throw RpcException.invalidParams("Option '" + key + "' has unknown value: " + value);
        }
    }
}
