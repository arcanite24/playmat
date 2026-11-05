package org.mage.test.serverside.trace;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import mage.abilities.Ability;
import mage.cards.Card;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.cards.repository.CardScanner;
import org.junit.Assume;
import org.junit.Test;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Card facts for the Rust engine's transpiler (docs/rust/Engine.md): for each card in a pool, its abilities in the
 * order {@code card.getAbilities()} lists them (the order decision logs number them in), with each one's class and
 * rule text as Java prints it ({@code getRule()}, the text a stack ability's name carries).
 * <p>
 * Opt-in: {@code -Dxmage.cardFacts=<out.json> -Dxmage.cardFacts.pool=<file, one card name per line>}.
 */
public class CardFactsTest {

    @Test
    public void exportCardFacts() throws Exception {
        String out = System.getProperty("xmage.cardFacts");
        String pool = System.getProperty("xmage.cardFacts.pool");
        Assume.assumeTrue("set -Dxmage.cardFacts=<file> and -Dxmage.cardFacts.pool=<file>", out != null && pool != null);
        CardScanner.scan();
        List<Map<String, Object>> cards = new ArrayList<>();
        for (String line : Files.readAllLines(new File(pool).toPath(), StandardCharsets.UTF_8)) {
            String name = line.trim();
            if (name.isEmpty()) {
                continue;
            }
            CardInfo info = CardRepository.instance.findPreferredCoreExpansionCard(name);
            if (info == null) {
                continue;
            }
            Card card = info.createCard();
            Map<String, Object> facts = new LinkedHashMap<>();
            facts.put("name", card.getName());
            List<Map<String, Object>> abilities = new ArrayList<>();
            for (Ability ability : card.getAbilities()) {
                Map<String, Object> a = new LinkedHashMap<>();
                a.put("class", ability.getClass().getSimpleName());
                a.put("type", ability.getAbilityType().name());
                a.put("rule", ability.getRule());
                abilities.add(a);
            }
            facts.put("abilities", abilities);
            cards.add(facts);
        }
        Gson gson = new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create();
        Files.write(new File(out).toPath(), gson.toJson(cards).getBytes(StandardCharsets.UTF_8));
    }
}
