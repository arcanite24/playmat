package mage.server.ladder;

import mage.cards.Card;
import mage.cards.decks.Deck;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.CardType;
import mage.game.Game;
import mage.game.permanent.Permanent;
import mage.game.stack.StackObject;
import mage.players.Player;
import org.apache.log4j.Logger;

import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Playbook-driven Bo3 sideboarding for AI Ranked opponents.
 * <p>
 * Records only publicly revealed opponent cards (battlefield, graveyard, exile, stack -- never hand or library),
 * classifies the opponent into a matchup tag (`vs_aggro`, `vs_control`, `vs_graveyard`,
 * `vs_artifacts_enchantments`, `vs_midrange`), and applies the bot's scripted `in`/`out` swaps between Bo3 games.
 */
public final class LadderSideboarder {

    private static final Logger logger = Logger.getLogger(LadderSideboarder.class);

    /** Publicly revealed human card names per live Bo3 ranked table */
    private static final Map<UUID, Set<String>> REVEALED = new ConcurrentHashMap<>();

    private LadderSideboarder() {
    }

    /** Records publicly revealed human cards from a Java-hosted game at the end of a Bo3 game. */
    public static void recordRevealedFromGame(UUID tableId, Game game) {
        if (tableId == null || game == null) {
            return;
        }
        Set<UUID> humans = new LinkedHashSet<>();
        for (Player p : game.getPlayers().values()) {
            if (p != null && p.isHuman()) {
                humans.add(p.getId());
            }
        }
        if (humans.isEmpty()) {
            return;
        }
        Set<String> names = new LinkedHashSet<>();
        for (Permanent perm : game.getBattlefield().getAllPermanents()) {
            if (humans.contains(perm.getOwnerId())) {
                names.add(perm.getName());
            }
        }
        for (UUID pid : humans) {
            Player p = game.getPlayer(pid);
            if (p == null) {
                continue;
            }
            for (Card c : p.getGraveyard().getCards(game)) {
                names.add(c.getName());
            }
        }
        for (Card c : game.getExile().getAllCards(game)) {
            if (humans.contains(c.getOwnerId())) {
                names.add(c.getName());
            }
        }
        for (StackObject s : game.getStack()) {
            if (humans.contains(s.getControllerId())) {
                names.add(s.getName());
            }
        }
        recordRevealedNames(tableId, names);
    }

    /** Records publicly revealed human card names reported by the Rust host at the end of a game. */
    public static void recordRevealedNames(UUID tableId, Collection<String> cardNames) {
        if (tableId == null || cardNames == null || cardNames.isEmpty()) {
            return;
        }
        REVEALED.computeIfAbsent(tableId, k -> ConcurrentHashMap.newKeySet()).addAll(cardNames);
    }

    /** Clears stored revealed-card history when a ranked match finishes. */
    public static void forget(UUID tableId) {
        if (tableId != null) {
            REVEALED.remove(tableId);
        }
    }

    /** Classifies publicly revealed opponent cards into a matchup tag for sideboard lookup. */
    public static String classify(Collection<String> revealedCardNames) {
        if (revealedCardNames == null || revealedCardNames.isEmpty()) {
            return "vs_midrange";
        }
        int creatures = 0;
        int creatureMvSum = 0;
        int burn = 0;
        int countersOrSweepers = 0;
        int graveyardSignals = 0;
        int artifactsOrEnchantments = 0;

        for (String name : revealedCardNames) {
            if (name == null || name.isEmpty()) {
                continue;
            }
            CardInfo info = CardRepository.instance.findPreferredCoreExpansionCard(name);
            if (info == null) {
                continue;
            }
            List<CardType> types = info.getTypes();
            if (types.contains(CardType.LAND)) {
                continue;
            }
            if (types.contains(CardType.CREATURE)) {
                creatures++;
                creatureMvSum += info.getManaValue();
            }
            if (types.contains(CardType.ARTIFACT) || types.contains(CardType.ENCHANTMENT)) {
                artifactsOrEnchantments++;
            }
            String rules = info.getRules() == null ? "" : String.join(" ", info.getRules()).toLowerCase(Locale.ENGLISH);
            if (rules.contains("counter target") || rules.contains("destroy all creatures") || rules.contains("exile all creatures")) {
                countersOrSweepers++;
            }
            if ((rules.contains("any target") || rules.contains("target player") || rules.contains("each opponent"))
                    && rules.contains("damage")) {
                burn++;
            }
            if (rules.contains("from your graveyard") || rules.contains("flashback") || rules.contains("unearth")
                    || rules.contains("delve") || rules.contains("mill ") || rules.contains("escape")) {
                graveyardSignals++;
            }
        }

        if (graveyardSignals >= 2) {
            return "vs_graveyard";
        }
        if (countersOrSweepers >= 2 || (creatures <= 1 && revealedCardNames.size() >= 6)) {
            return "vs_control";
        }
        double avgCreatureMv = creatures > 0 ? (double) creatureMvSum / creatures : 9.0;
        if (burn >= 2 || (creatures >= 3 && avgCreatureMv <= 2.4)) {
            return "vs_aggro";
        }
        if (artifactsOrEnchantments >= 4) {
            return "vs_artifacts_enchantments";
        }
        return "vs_midrange";
    }

    /**
     * Applies the bot's scripted Bo3 sideboard swaps to `deck` based on the opponent cards revealed at `tableId`.
     * Preserves exact maindeck size; returns the number of cards swapped in.
     */
    public static int apply(UUID tableId, String playbookId, Deck deck) {
        if (tableId == null || playbookId == null || deck == null) {
            return 0;
        }
        LadderPlaybooks.DeckPlaybook pb = LadderPlaybooks.get().deck(playbookId);
        if (pb == null || pb.sideboard == null || pb.sideboard.isEmpty()) {
            return 0;
        }
        Set<String> seen = REVEALED.getOrDefault(tableId, Collections.emptySet());
        String matchup = classify(seen);
        LadderPlaybooks.SideboardSwap plan = pb.sideboard.get(matchup);
        if (plan == null && !"vs_midrange".equals(matchup)) {
            plan = pb.sideboard.get("vs_midrange");
        }
        if (plan == null || plan.in == null || plan.out == null) {
            return 0;
        }
        int swapped = applyPlan(deck, plan);
        if (swapped > 0) {
            logger.info("Ranked Bo3 sideboard (" + tableId + "): " + playbookId + " classified " + matchup
                    + " (" + seen.size() + " seen cards), swapped " + swapped + " card(s)");
        }
        return swapped;
    }

    /** Executes one `in`/`out` swap plan on `deck`, moving equal numbers of cards between maindeck and sideboard. */
    static int applyPlan(Deck deck, LadderPlaybooks.SideboardSwap plan) {
        if (deck == null || plan == null || plan.in == null || plan.out == null) {
            return 0;
        }
        List<Card> toBringIn = pickByName(deck.getSideboard(), plan.in);
        List<Card> toTakeOut = pickByName(deck.getCards(), plan.out);
        int count = Math.min(toBringIn.size(), toTakeOut.size());
        if (count == 0) {
            return 0;
        }
        for (int i = 0; i < count; i++) {
            Card inCard = toBringIn.get(i);
            Card outCard = toTakeOut.get(i);
            deck.getSideboard().remove(inCard);
            deck.getCards().remove(outCard);
            deck.getCards().add(inCard);
            deck.getSideboard().add(outCard);
        }
        return count;
    }

    private static List<Card> pickByName(Collection<Card> zone, Map<String, Integer> wanted) {
        Map<String, Integer> remaining = new LinkedHashMap<>(wanted);
        List<Card> picked = new ArrayList<>();
        for (Card c : zone) {
            Integer need = remaining.get(c.getName());
            if (need != null && need > 0) {
                picked.add(c);
                remaining.put(c.getName(), need - 1);
            }
        }
        return picked;
    }
}
