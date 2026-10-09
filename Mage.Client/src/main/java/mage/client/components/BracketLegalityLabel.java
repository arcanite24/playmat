package mage.client.components;

import mage.cards.decks.CommanderBrackets;
import mage.cards.decks.Deck;
import mage.client.util.GUISizeHelper;

import java.awt.*;
import java.util.ArrayList;
import java.util.List;

/**
 * Inject bracket level inside validation panel
 * See more details at <a href="https://mtg.wiki/page/Commander_Brackets">wiki</a>
 * <p>
 * Support:
 * - [x] game changers
 * - [x] infinite combos
 * - [x] mass land destruction
 * - [x] extra turns
 * - [x] tutors
 * Features:
 * - [x] find possible bracket level of the deck
 * - [x] find affected cards by checking group
 * - [x] can auto-generate infinite combos list, see verify test downloadAndPrepareCommanderBracketsData
 * - [x] rules are shared with the web client, see {@link CommanderBrackets}
 * - [ ] TODO: tests
 * - [ ] TODO: table - players brackets level disclose settings
 * - [ ] TODO: generate - convert card name to xmage format and assert on bad names (ascii only)
 *
 * @author JayDi85
 */
public class BracketLegalityLabel extends LegalityLabel {

    private final String fullName;
    private final String shortName;
    private final int maxLevel;

    private final List<String> badCards = new ArrayList<>();

    public BracketLegalityLabel(String fullName, String shortName, int maxLevel) {
        super(shortName, null);
        this.fullName = fullName;
        this.shortName = shortName;
        this.maxLevel = maxLevel;
        setPreferredSize(DIM_PREFERRED_1_OF_5);
    }

    @Override
    public List<String> selectCards() {
        return new ArrayList<>(this.badCards);
    }

    @Override
    public void validateDeck(Deck deck) {
        CommanderBrackets.Analysis analysis = CommanderBrackets.analyze(deck);
        this.badCards.clear();
        this.badCards.addAll(analysis.getBadCards(this.maxLevel));

        int infoFontHeaderSize = Math.round(GUISizeHelper.cardTooltipFont.getSize() * 1.0f);
        int infoFontTextSize = Math.round(GUISizeHelper.cardTooltipFont.getSize() * 0.6f);

        // show all found cards in any use cases
        Color showColor = this.badCards.isEmpty() ? COLOR_LEGAL : COLOR_NOT_LEGAL;

        List<String> showInfo = new ArrayList<>();
        if (this.badCards.isEmpty()) {
            showInfo.add(String.format("<span style='font-weight:bold;font-size:%dpx;'><p>Deck is <span style='color:green;'>GOOD</span> for %s</p></span>",
                    infoFontHeaderSize,
                    this.fullName
            ));
        } else {
            showInfo.add(String.format("<span style='font-weight:bold;font-size:%dpx;'><p>Deck is <span style='color:#BF544A;'>BAD</span> for %s</p></span>",
                    infoFontHeaderSize,
                    this.fullName
            ));
            showInfo.add("<p>(click here to select all bad cards)</p>");
        }

        analysis.getGroups().forEach((groupName, cards) -> {
            String group = groupName + getStats(groupName, cards.size());
            showInfo.add("<br>");
            showInfo.add("<span style='font-weight:bold;font-size: " + infoFontTextSize + "px;'>" + group + "</span>");
            showInfo.add("<ul style=\"font-size: " + infoFontTextSize + "px; width: " + TOOLTIP_TABLE_WIDTH + "px; padding-left: 10px; margin: 0;\">");
            if (cards.isEmpty()) {
                showInfo.add("<li style=\"margin-bottom: 2px;\">no cards</li>");
            } else {
                cards.forEach(s -> showInfo.add(String.format("<li style=\"margin-bottom: 2px;\">%s</li>", s)));
            }
            showInfo.add("</ul>");
        });

        String showText = "<html><body>" + String.join("\n", showInfo) + "</body></html>";
        showState(showColor, showText, false);
    }

    private String getStats(String groupName, int currentAmount) {
        int maxAmount = CommanderBrackets.getMaxCards(groupName, this.maxLevel);
        String info;
        if (currentAmount > maxAmount) {
            info = " (<span style='color:#BF544A;'>%s of %s</span>)";
        } else {
            info = " (<span>%s of %s</span>)";
        }
        return String.format(info, currentAmount, maxAmount == CommanderBrackets.ANY_AMOUNT ? "any" : maxAmount);
    }
}
