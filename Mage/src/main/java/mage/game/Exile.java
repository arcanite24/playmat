package mage.game;

import mage.abilities.Ability;
import mage.cards.Card;
import mage.filter.FilterCard;
import mage.util.Copyable;

import java.io.IOException;
import java.io.ObjectInputStream;
import java.io.Serializable;
import java.util.*;
import java.util.Map.Entry;
import java.util.stream.Collectors;

/**
 * @author BetaSteward_at_googlemail.com
 */
public class Exile implements Serializable, Copyable<Exile> {

    // fixed, not random per JVM: a game saved by one server process (GameSnapshot) is read back by another
    private static final UUID PERMANENT = UUID.fromString("6f1c2b8e-0d5a-4c7e-9a3b-5e2d8f4a1c90");
    private static final String PERMANENT_NAME = "Permanent";

    private final Map<UUID, ExileZone> exileZones = new HashMap<>();

    public Exile() {
        createZone(PERMANENT, PERMANENT_NAME);
    }

    protected Exile(final Exile exile) {
        for (Entry<UUID, ExileZone> entry : exile.exileZones.entrySet()) {
            exileZones.put(entry.getKey(), entry.getValue().copy());
        }
    }

    public Collection<ExileZone> getExileZones() {
        return exileZones.values();
    }

    public ExileZone getPermanentExile() {
        return exileZones.get(PERMANENT);
    }

    public void add(UUID id, String name, Card card) {
        createZone(id, name).add(card);
    }

    public void add(Card card) {
        exileZones.get(PERMANENT).add(card);
    }

    public ExileZone createZone(UUID id, String name) {
        return exileZones.computeIfAbsent(id, x -> new ExileZone(id, name + " - Exile"));
    }

    public ExileZone getExileZone(UUID id) {
        return exileZones.get(id);
    }

    public Card getCard(UUID cardId, Game game) {
        for (ExileZone exile : exileZones.values()) {
            if (exile.contains(cardId)) {
                return game.getCard(cardId);
            }
        }
        return null;
    }

    /**
     * Returns all cards in exile matching the filter. Use only for test framework.
     * For card effects, instead use a method that checks owner or range of influence.
     */
    @Deprecated
    public List<Card> getCards(FilterCard filter, Game game) {
        List<Card> allCards = getAllCards(game);
        return allCards.stream().filter(card -> filter.match(card, game)).collect(Collectors.toList());
    }

    /**
     * Returns all cards in exile. Use only for test framework.
     * For card effects, instead use a method that checks owner or range of influence.
     */
    @Deprecated
    public List<Card> getAllCards(Game game) {
        return getCardsOwned(game, null);
    }

    /**
     * Returns all cards in exile owned by the specified player
     */
    public List<Card> getCardsOwned(Game game, UUID ownerId) {
        List<Card> res = new ArrayList<>();
        for (ExileZone exile : exileZones.values()) {
            for (Card card : exile.getCards(game)) {
                if (ownerId == null || card.isOwnedBy(ownerId)) {
                    res.add(card);
                }
            }
        }
        return res;
    }

    /**
     * Returns all cards in exile matching the filter, owned by the specified player
     */
    public Set<Card> getCardsOwned(FilterCard filter, UUID playerId, Ability source, Game game) {
        return getCardsOwned(game, playerId)
                .stream()
                .filter(card -> filter.match(card, playerId, source, game))
                .collect(Collectors.toCollection(LinkedHashSet::new));
    }

    /**
     * Returns all cards in exile in range of the specified player
     */
    public List<Card> getCardsInRange(Game game, UUID controllerId) {
        List<Card> res = new ArrayList<>();
        for (UUID playerId : game.getState().getPlayersInRange(controllerId, game)) {
            res.addAll(getCardsOwned(game, playerId));
        }
        return res;
    }

    /**
     * Returns all cards in exile matching the filter, in range of the specified player
     */
    public Set<Card> getCardsInRange(FilterCard filter, UUID playerId, Ability source, Game game) {
        return getCardsInRange(game, playerId)
                .stream()
                .filter(card -> filter.match(card, playerId, source, game))
                .collect(Collectors.toCollection(LinkedHashSet::new));
    }

    public boolean removeCard(Card card) {
        for (ExileZone exile : exileZones.values()) {
            if (exile.contains(card.getId())) {
                return exile.remove(card.getId());
            }
        }
        return false;
    }

    /**
     * Move card from one exile zone to another. Use case example: create special zone for exiled and castable card.
     */
    public void moveToAnotherZone(Card card, Game game, ExileZone exileZone) {
        if (getCard(card.getId(), game) == null) {
            throw new IllegalArgumentException("Card must be in exile zone: " + card.getIdName());
        }
        if (exileZone == null) {
            throw new IllegalArgumentException("Exile zone must exists: " + card.getIdName());
        }
        removeCard(card);
        exileZone.add(card);
    }

    public void moveToMainExileZone(Card card, Game game) {
        moveToAnotherZone(card, game, getExileZone(PERMANENT));
    }

    private void readObject(ObjectInputStream in) throws IOException, ClassNotFoundException {
        in.defaultReadObject();
        if (exileZones.containsKey(PERMANENT)) {
            return;
        }
        // saved before the id was fixed: the main zone is under the old process's random id
        ExileZone main = createZone(PERMANENT, PERMANENT_NAME);
        String mainName = main.getName();
        Iterator<Entry<UUID, ExileZone>> it = exileZones.entrySet().iterator();
        while (it.hasNext()) {
            ExileZone zone = it.next().getValue();
            if (zone != main && mainName.equals(zone.getName())) {
                main.addAll(zone);
                it.remove();
            }
        }
    }

    @Override
    public Exile copy() {
        return new Exile(this);
    }

    public boolean containsId(UUID cardId, Game game) {
        for (Card card : getAllCards(game)) {
            if (card.getId().equals(cardId)) {
                return true;
            }
        }
        return false;
    }

    public void clear() {
        for (ExileZone exile : exileZones.values()) {
            exile.clear();
        }
    }

    public void cleanupEndOfTurnZones(Game game) {
        // moves cards from outdated zone to main exile zone
        ExileZone mainZone = getExileZone(PERMANENT);
        for (ExileZone zone : exileZones.values()) {
            if (zone.isCleanupOnEndTurn()) {
                for (Card card : zone.getCards(game)) {
                    mainZone.add(card);
                    zone.remove(card);
                }
            }
        }
    }

    @Override
    public String toString() {
        return "Cards: " + exileZones.values()
                .stream()
                .mapToInt(ExileZone::size)
                .sum();
    }
}
