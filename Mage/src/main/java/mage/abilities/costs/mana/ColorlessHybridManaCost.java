package mage.abilities.costs.mana;

import mage.Mana;
import mage.abilities.Ability;
import mage.abilities.costs.Cost;
import mage.constants.ColoredManaSymbol;
import mage.constants.ManaType;
import mage.game.Game;
import mage.players.ManaPool;

import java.util.ArrayList;
import java.util.List;

public class ColorlessHybridManaCost extends ManaCostImpl {

    private final ColoredManaSymbol manaColor;

    public ColorlessHybridManaCost(ColoredManaSymbol manaColor) {
        this.manaColor = manaColor;
        this.cost = new Mana(manaColor);
        this.cost.add(Mana.ColorlessMana(1));
        addColoredOption(manaColor);
        options.add(Mana.ColorlessMana(1));
    }

    public ColorlessHybridManaCost(ColorlessHybridManaCost manaCost) {
        super(manaCost);
        this.manaColor = manaCost.manaColor;
    }

    @Override
    public int manaValue() {
        return 1;
    }

    @Override
    public boolean isPaid() {
        return paid || isColoredPaid(this.manaColor) || isColorlessPaid(1);
    }

    @Override
    public void assignPayment(Game game, Ability ability, ManaPool pool, Cost costToPay) {
        // Assign colorless first in an attempt to avoid things like pain lands and talismans causing damage
        if (!assignColorless(ability, game, pool, 1, costToPay)) {
            assignColored(ability, game, pool, manaColor, costToPay);
        }
    }

    /**
     * paid by {C} alone, so its {C} can go to another colorless hybrid symbol once its color pays it instead
     */
    public boolean isPaidWithColorlessOnly() {
        return payment.getColorless() > 0 && payment.getColor(manaColor) == 0;
    }

    /**
     * moves this symbol's {C} payment to another, unpaid one (this one is paid by its color by then)
     */
    void giveColorlessTo(ColorlessHybridManaCost other) {
        payment.decrease(ManaType.COLORLESS);
        other.payment.increaseColorless();
        if (usedManaToPay.getColorless() > 0) {
            usedManaToPay.decrease(ManaType.COLORLESS);
            other.usedManaToPay.increaseColorless();
        }
    }

    @Override
    public String getText() {
        return "{C/" + manaColor.toString() + '}';
    }

    @Override
    public ColorlessHybridManaCost getUnpaid() {
        return this;
    }

    @Override
    public boolean testPay(Mana testMana) {
        return testMana.getColor(manaColor) > 0 || testMana.getColorless() > 0;
    }

    @Override
    public ColorlessHybridManaCost copy() {
        return new ColorlessHybridManaCost(this);
    }

    @Override
    public boolean containsColor(ColoredManaSymbol coloredManaSymbol) {
        return manaColor == coloredManaSymbol;
    }

    public ColoredManaSymbol getManaColor() {
        return manaColor;
    }

    @Override
    public List<Mana> getManaOptions() {
        List<Mana> manaList = new ArrayList<>();
        manaList.add(new Mana(manaColor));
        manaList.add(Mana.ColorlessMana(1));
        return manaList;
    }
}
