package org.mage.test.serverside.trace;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import mage.abilities.Abilities;
import mage.abilities.Ability;
import mage.cards.PrepareCard;
import mage.cards.SplitCard;
import mage.cards.DoubleFacedCard;
import mage.constants.MultiplayerAttackOption;
import mage.constants.RangeOfInfluence;
import mage.game.TwoPlayerDuel;
import mage.game.mulligan.MulliganType;
import mage.game.permanent.PermanentCard;
import mage.util.CardUtil;
import mage.cards.Card;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.cards.repository.TokenInfo;
import mage.cards.repository.TokenRepository;
import mage.cards.repository.CardScanner;
import mage.game.Game;
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
import java.util.Arrays;
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

    /** the owner and controller of the permanents whose views it writes (their ids aren't in the templates) */
    private static final java.util.UUID SEAT = java.util.UUID.nameUUIDFromBytes(new byte[]{0});

    /**
     * A template's ids as references relative to the object's own ({@code $}, as {@code DecisionLog.refs} names an
     * object's faces: {@code $f0}, {@code $f1}, a Room's halves {@code $h0}, {@code $h1}), its owner's {@code $P};
     * any other becomes {@code ?}. The engine puts the object's reference in place of {@code $}.
     */
    private static Map<java.util.UUID, String> refs(Card card) {
        Map<java.util.UUID, String> refs = new java.util.HashMap<>();
        refs.put(SEAT, "$P");
        Card main = card.getMainCard();
        if (main != null && !main.getId().equals(card.getId())) {
            refs.put(main.getId(), "$f0");
        }
        if (card instanceof DoubleFacedCard) {
            refs.put(((DoubleFacedCard) card).getLeftHalfCard().getId(), "$f0");
            refs.put(((DoubleFacedCard) card).getRightHalfCard().getId(), "$f1");
        }
        if (card instanceof mage.cards.CardWithSpellOption) {
            refs.put(((mage.cards.CardWithSpellOption) card).getSpellCard().getId(), "$f1");
        }
        if (card instanceof SplitCard) {
            refs.put(((SplitCard) card).getLeftHalfCard().getId(), "$h0");
            refs.put(((SplitCard) card).getRightHalfCard().getId(), "$h1");
        }
        refs.put(card.getId(), "$");
        return refs;
    }

    @Test
    public void exportCardFacts() throws Exception {
        String out = System.getProperty("xmage.cardFacts");
        String pool = System.getProperty("xmage.cardFacts.pool");
        Assume.assumeTrue("set -Dxmage.cardFacts=<file> and -Dxmage.cardFacts.pool=<file>", out != null && pool != null);
        CardScanner.scan();
        List<Map<String, Object>> cards = new ArrayList<>();
        String viewsOut = System.getProperty("xmage.cardFacts.views");
        Map<String, Object> views = new TreeMap<>();
        // a game that hasn't started: a permanent's view in it has no hints (`CardUtil.getCardRulesWithAdditionalInfo`)
        Game unstarted = new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL,
                MulliganType.GAME_DEFAULT.getMulligan(0), 60, 20, 7);
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
                Token token = (Token) cls.getConstructor().newInstance();
                cardAbilities = token.getAbilities();
                images = images(cls);
                if (viewsOut != null) {
                    // its view, and its images in the order `findPreferredTokenInfo` picks from
                    Map<String, Object> v = new LinkedHashMap<>();
                    v.put("view", ShadowViews.mapIds(JsonCodec.GSON.toJsonTree(new CardView(token, (Game) null)),
                            new java.util.HashMap<>()));
                    List<List<Object>> list = new ArrayList<>();
                    for (TokenInfo info : TokenRepository.instance.getByClassName(cls.getName())) {
                        list.add(Arrays.asList(info.getSetCode(), info.getImageNumber()));
                    }
                    v.put("images", list);
                    views.put(name, v);
                }
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
                    views.put(name, ShadowViews.mapIds(JsonCodec.GSON.toJsonTree(new CardView(card)), refs(card)));
                    // a modal ability's modes' texts, by the ability's index (`Chosen mode: …` on the stack)
                    Map<String, List<String>> modes = new LinkedHashMap<>();
                    int index = 0;
                    for (mage.abilities.Ability a : card.getAbilities()) {
                        if (a.isModal()) {
                            List<String> texts = new ArrayList<>();
                            for (mage.abilities.Mode mode : a.getModes().values()) {
                                texts.add(mode.getEffects().getText(mode));
                            }
                            modes.put(String.valueOf(index), texts);
                        }
                        index++;
                    }
                    if (!modes.isEmpty()) {
                        views.put("modes:" + name, modes);
                    }
                    // each face it can be on the battlefield as, as a permanent's view (`PermanentView`'s card part):
                    // its front, and a double-faced card's back (an MDFC's played face, a transformed card)
                    List<Card> faces = new ArrayList<>();
                    faces.add(CardUtil.getDefaultCardSideForBattlefield(unstarted, card));
                    if (card instanceof DoubleFacedCard) {
                        faces.add(((DoubleFacedCard) card).getRightHalfCard());
                    }
                    for (Card face : faces) {
                        if (!face.isPermanent()) {
                            continue;
                        }
                        // a face's own view: what a permanent of it shows as its card (`PermanentView.original`)
                        if (face != card) {
                            views.put("face:" + face.getName(),
                                    ShadowViews.mapIds(JsonCodec.GSON.toJsonTree(new CardView(face.copy(), (Game) null)), refs(face)));
                        }
                        try {
                            face.setOwnerId(SEAT);
                            PermanentCard permanent = new PermanentCard(face, SEAT, unstarted);
                            views.put("permanent:" + permanent.getName(), ShadowViews.mapIds(
                                    JsonCodec.GSON.toJsonTree(new CardView(permanent, unstarted, true)), refs(permanent)));
                        } catch (RuntimeException e) {
                            System.err.println("no permanent view of " + face.getName() + ": " + e);
                        }
                    }
                    // the spell of each part cast on its own (a Room's halves, an adventure's or omen's spell), as it is
                    // on the stack (`CardView(Spell)`)
                    List<Card> parts = new ArrayList<>();
                    if (card instanceof SplitCard) {
                        parts.add(((SplitCard) card).getLeftHalfCard());
                        parts.add(((SplitCard) card).getRightHalfCard());
                    }
                    if (card instanceof mage.cards.CardWithSpellOption) {
                        parts.add(((mage.cards.CardWithSpellOption) card).getSpellCard());
                        // and the card itself cast (an adventurer, a prepare creature): not split as a spell
                        parts.add(card);
                    }
                    for (Card part : parts) {
                        try {
                            part.setOwnerId(SEAT);
                            mage.game.stack.Spell spell = new mage.game.stack.Spell(part, part.getSpellAbility(), SEAT,
                                    mage.constants.Zone.HAND, unstarted);
                            // (without the game: the spell's characteristics are its part's)
                            views.put("spell:" + part.getName(), ShadowViews.mapIds(
                                    JsonCodec.GSON.toJsonTree(new CardView(spell, (Game) null, true)), refs(part)));
                        } catch (RuntimeException e) {
                            System.err.println("no spell view of " + part.getName() + ": " + e);
                        }
                    }
                    // a prepare card's spell, as the copy it puts in exile once the permanent is prepared
                    if (card instanceof PrepareCard) {
                        try {
                            PrepareCard prepare = (PrepareCard) card;
                            Card copy = prepare.createPreparedSpellCopy(SEAT);
                            unstarted.getState().addCardPartCopyToExileZone(card, prepare.getSpellCard(), copy, SEAT);
                            views.put("prepared:" + copy.getName(),
                                    ShadowViews.mapIds(JsonCodec.GSON.toJsonTree(new CardView(copy)), refs(copy)));
                        } catch (RuntimeException e) {
                            System.err.println("no prepared copy of " + name + ": " + e);
                        }
                    }
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
            if (!ability.getRuleVisible()) {
                a.put("rule_hidden", true);
            }
            if (ability.getRuleAtTheTop()) {
                a.put("rule_at_top", true);
            }
            // the hints its card's view adds in a game (`CardUtil.getCardRulesWithAdditionalInfo`), each with the
            // fields that make it: its class, and for each field the value's class and text
            if (!ability.getHints().isEmpty()) {
                List<Map<String, Object>> hints = new ArrayList<>();
                for (mage.abilities.hint.Hint hint : ability.getHints()) {
                    hints.add(describe(hint));
                }
                a.put("hints", hints);
            }
            abilities.add(a);
        }
        return abilities;
    }

    /** an object's class and fields (each field's value: its class, and its text or own fields), for the facts */
    private static Map<String, Object> describe(Object o) {
        Map<String, Object> d = new LinkedHashMap<>();
        d.put("class", o.getClass().getSimpleName());
        for (Class<?> c = o.getClass(); c != null && c != Object.class; c = c.getSuperclass()) {
            for (java.lang.reflect.Field f : c.getDeclaredFields()) {
                if (java.lang.reflect.Modifier.isStatic(f.getModifiers())) {
                    continue;
                }
                try {
                    f.setAccessible(true);
                    Object v = f.get(o);
                    if (v == null) {
                        continue;
                    }
                    if (v instanceof String || v instanceof Number || v instanceof Boolean || v instanceof Enum) {
                        d.put(f.getName(), String.valueOf(v));
                    } else if (v instanceof java.awt.Color) {
                        d.put(f.getName(), "#" + Integer.toHexString(((java.awt.Color) v).getRGB() & 0xffffff));
                    } else {
                        Map<String, Object> x = new LinkedHashMap<>();
                        x.put("class", v.getClass().getName());
                        x.put("text", String.valueOf(v));
                        d.put(f.getName(), x);
                    }
                } catch (Exception e) {
                    d.put(f.getName(), "?");
                }
            }
        }
        return d;
    }

    private static Map<String, Integer> images(Class<?> cls) {
        Map<String, Integer> images = new TreeMap<>();
        for (TokenInfo info : TokenRepository.instance.getByClassName(cls.getName())) {
            images.merge(info.getSetCode(), 1, Integer::sum);
        }
        return images;
    }
}
