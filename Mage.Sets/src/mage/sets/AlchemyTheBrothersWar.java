package mage.sets;

import mage.cards.ExpansionSet;
import mage.constants.Rarity;
import mage.constants.SetType;

/**
 * https://scryfall.com/sets/ybro
 *
 * @author playmat
 */
public final class AlchemyTheBrothersWar extends ExpansionSet {

    private static final AlchemyTheBrothersWar instance = new AlchemyTheBrothersWar();

    public static AlchemyTheBrothersWar getInstance() {
        return instance;
    }

    private AlchemyTheBrothersWar() {
        super("Alchemy: The Brothers' War", "YBRO", ExpansionSet.buildDate(2022, 12, 13), SetType.MAGIC_ARENA);
        this.blockName = "Alchemy";
        this.hasBoosters = false;
        this.hasBasicLands = false;

        cards.add(new SetCardInfo("Fallaji Antiquarian", 8, Rarity.RARE, mage.cards.f.FallajiAntiquarian.class));
        cards.add(new SetCardInfo("Gixian Recycler", 27, Rarity.RARE, mage.cards.g.GixianRecycler.class));
    }
}
