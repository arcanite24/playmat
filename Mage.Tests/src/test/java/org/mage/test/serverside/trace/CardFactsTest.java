package org.mage.test.serverside.trace;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import mage.abilities.Abilities;
import mage.abilities.Ability;
import mage.cards.SplitCard;
import mage.cards.Card;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.cards.repository.TokenInfo;
import mage.cards.repository.TokenRepository;
import mage.cards.repository.CardScanner;
import mage.game.command.Emblem;
import mage.game.permanent.token.Token;
import mage.server.websocket.rpc.JsonCodec;
import mage.view.CardView;
import org.junit.Assume;
import org.junit.Test;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

/**
 * Card facts for the Rust engine's transpiler (docs/rust/Engine.md): for each card in a pool, its abilities in the
 * order {@code card.getAbilities()} lists them (the order decision logs number them in), with each one's class and
 * rule text as Java prints it ({@code getRule()}, the text a stack ability's name carries).
 * <p>
 * A pool line {@code token:<class>} names a token class from {@code mage.game.permanent.token} instead, and
 * {@code emblem:<class>} an emblem class from {@code mage.game.command.emblems}.
 * <p>
 * Opt-in: {@code -Dxmage.cardFacts=<out.json> -Dxmage.cardFacts.pool=<file, one card name per line>}. With
 * {@code -Dxmage.cardFacts.views=<views.json>} it also writes each card's view as the web client gets it out of a
 * game ({@code new CardView(card)}, as {@code JsonCodec} writes it), the template the Rust engine's views start from
 * (docs/rust/Views.md).
 */
public class CardFactsTest {

    @Test
    public void exportCardFacts() throws Exception {
        String out = System.getProperty("xmage.cardFacts");
        String pool = System.getProperty("xmage.cardFacts.pool");
        Assume.assumeTrue("set -Dxmage.cardFacts=<file> and -Dxmage.cardFacts.pool=<file>", out != null && pool != null);
        CardScanner.scan();
        List<Map<String, Object>> cards = new ArrayList<>();
        String viewsOut = System.getProperty("xmage.cardFacts.views");
        Map<String, Object> views = new TreeMap<>();
        for (String line : Files.readAllLines(new File(pool).toPath(), StandardCharsets.UTF_8)) {
            String name = line.trim();
            if (name.isEmpty()) {
                continue;
            }
            Abilities<Ability> cardAbilities;
            List<Map<String, Object>> halves = null;
            // a token's or emblem's images per set: creating one picks an image at random (`TokenRepository`'s
            // `findPreferredTokenInfo`, the game's random) when its source's set, or else all sets, has two or more
            Map<String, Integer> images = null;
            String set = null;
            if (name.startsWith("token:")) {
                // a token class from mage.game.permanent.token, by its simple name
                Class<?> cls = Class.forName("mage.game.permanent.token." + name.substring("token:".length()));
                cardAbilities = ((Token) cls.getConstructor().newInstance()).getAbilities();
                images = images(cls);
            } else if (name.startsWith("emblem:")) {
                // an emblem class from mage.game.command.emblems
                Class<?> cls = Class.forName("mage.game.command.emblems." + name.substring("emblem:".length()));
                cardAbilities = ((Emblem) cls.getConstructor().newInstance()).getAbilities();
                images = images(cls);
            } else {
                CardInfo info = CardRepository.instance.findPreferredCoreExpansionCard(name);
                if (info == null) {
                    continue;
                }
                Card card = info.createCard();
                name = card.getName();
                cardAbilities = card.getAbilities();
                // a split card's (a Room's) halves, each with its own abilities
                if (card instanceof SplitCard) {
                    halves = new ArrayList<>();
                    for (Card half : new Card[]{((SplitCard) card).getLeftHalfCard(), ((SplitCard) card).getRightHalfCard()}) {
                        Map<String, Object> h = new LinkedHashMap<>();
                        h.put("name", half.getName());
                        h.put("abilities", abilityFacts(half.getAbilities()));
                        halves.add(h);
                    }
                }
                // the printing a deck by name gets (`TxtDeckImporter`), whose set the tokens it makes look for
                set = card.getExpansionSetCode();
                if (viewsOut != null) {
                    views.put(name, JsonCodec.GSON.toJsonTree(new CardView(card)));
                }
            }
            Map<String, Object> facts = new LinkedHashMap<>();
            facts.put("name", name);
            if (set != null) {
                facts.put("set", set);
            }
            if (images != null) {
                facts.put("images", images);
            }
            facts.put("abilities", abilityFacts(cardAbilities));
            if (halves != null) {
                facts.put("halves", halves);
            }
            cards.add(facts);
        }
        Gson gson = new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create();
        Files.write(new File(out).toPath(), gson.toJson(cards).getBytes(StandardCharsets.UTF_8));
        if (viewsOut != null) {
            Gson compact = new GsonBuilder().disableHtmlEscaping().create();
            Files.write(new File(viewsOut).toPath(), compact.toJson(views).getBytes(StandardCharsets.UTF_8));
        }
    }

    private static List<Map<String, Object>> abilityFacts(Iterable<Ability> cardAbilities) {
        List<Map<String, Object>> abilities = new ArrayList<>();
        for (Ability ability : cardAbilities) {
            Map<String, Object> a = new LinkedHashMap<>();
            a.put("class", ability.getClass().getSimpleName());
            a.put("type", ability.getAbilityType().name());
            a.put("rule", ability.getRule());
            abilities.add(a);
        }
        return abilities;
    }

    private static Map<String, Integer> images(Class<?> cls) {
        Map<String, Integer> images = new TreeMap<>();
        for (TokenInfo info : TokenRepository.instance.getByClassName(cls.getName())) {
            images.merge(info.getSetCode(), 1, Integer::sum);
        }
        return images;
    }
}
