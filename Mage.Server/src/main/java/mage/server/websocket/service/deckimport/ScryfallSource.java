package mage.server.websocket.service.deckimport;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.net.URI;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Web client bridge: Scryfall decks, from the API's JSON export ({@code /decks/{id}/export/json}).
 * Entries come grouped by section: commanders, the deck itself (split into nonlands and lands, or mainboard),
 * sideboard or outside (cards from outside the game, such as a companion) and maybeboard.
 */
final class ScryfallSource implements DeckSource {

    private static final Pattern DECK_PATH = Pattern.compile(
            "^/(?:@[^/]+/)?decks/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})(?:/.*)?$");
    private static final Pattern AUTHOR = Pattern.compile("/@([^/]+)/decks/");

    @Override
    public String id() {
        return "scryfall";
    }

    @Override
    public List<String> pageHosts() {
        return Arrays.asList("scryfall.com", "www.scryfall.com", "api.scryfall.com");
    }

    @Override
    public List<String> fetchHosts() {
        return Arrays.asList("api.scryfall.com");
    }

    @Override
    public String deckId(URI url) {
        Matcher matcher = DECK_PATH.matcher(url.getPath() == null ? "" : url.getPath());
        return matcher.matches() ? matcher.group(1).toLowerCase(java.util.Locale.ROOT) : null;
    }

    @Override
    public ImportedDeck fetch(URI url, String deckId, Fetcher fetcher) throws DeckImportException {
        String body = fetcher.get(URI.create("https://api.scryfall.com/decks/" + deckId + "/export/json"), "application/json");
        return read(body, deckId);
    }

    static ImportedDeck read(String body, String deckId) throws DeckImportException {
        JsonObject json = SourceSupport.parseObject(body, "Scryfall");
        if ("error".equals(SourceSupport.string(json, "object"))) {
            throw new DeckImportException(DeckImportException.Reason.NOT_FOUND, "Scryfall has no deck " + deckId);
        }
        JsonObject entries = SourceSupport.object(json, "entries");
        if (!"deck".equals(SourceSupport.string(json, "object")) || entries == null) {
            throw new DeckImportException(DeckImportException.Reason.SITE_CHANGED, "Scryfall's deck export has no entries");
        }
        String page = SourceSupport.string(json, "scryfall_uri");
        ImportedDeck deck = new ImportedDeck("scryfall", deckId, page != null ? page : "https://scryfall.com/decks/" + deckId);
        deck.setName(SourceSupport.string(json, "name"));
        deck.setFormat(SourceSupport.deckType(SourceSupport.string(json, "format")));
        deck.setDescription(SourceSupport.string(json, "description"));
        if (page != null) {
            Matcher author = AUTHOR.matcher(page);
            if (author.find()) {
                deck.setAuthor(author.group(1));
            }
        }

        for (Map.Entry<String, JsonElement> group : entries.entrySet()) {
            ImportedDeck.Section section = sectionOf(group.getKey());
            if (!group.getValue().isJsonArray()) {
                continue;
            }
            for (JsonElement element : group.getValue().getAsJsonArray()) {
                if (!element.isJsonObject()) {
                    continue;
                }
                JsonObject entry = element.getAsJsonObject();
                JsonObject card = SourceSupport.object(entry, "card_digest");
                if (card == null) {
                    // a line Scryfall couldn't match to a card (an empty row, or a typo)
                    continue;
                }
                String name = SourceSupport.cardName(SourceSupport.string(card, "name"));
                String setCode = SourceSupport.string(card, "set");
                String number = SourceSupport.string(card, "collector_number");
                deck.add(section, name, setCode, number, SourceSupport.integer(entry, "count", 0));
                if (section == ImportedDeck.Section.COMMANDER && deck.getCover() == null) {
                    deck.setCover(name, setCode, number);
                }
            }
        }
        if (deck.cardCount() == 0) {
            throw new DeckImportException(DeckImportException.Reason.NOT_FOUND, "Scryfall's deck " + deckId + " has no cards");
        }
        return deck;
    }

    private static ImportedDeck.Section sectionOf(String name) {
        switch (name) {
            case "commanders":
                return ImportedDeck.Section.COMMANDER;
            case "outside":
                return ImportedDeck.Section.COMPANION;
            case "sideboard":
                return ImportedDeck.Section.SIDE;
            case "maybeboard":
                return ImportedDeck.Section.MAYBE;
            default:
                return ImportedDeck.Section.MAIN;
        }
    }
}
