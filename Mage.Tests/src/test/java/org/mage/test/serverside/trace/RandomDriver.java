package org.mage.test.serverside.trace;

import mage.abilities.ActivatedAbility;
import mage.abilities.PlayLandAbility;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.events.PlayerQueryEvent;
import mage.game.permanent.Permanent;
import mage.player.human.HumanPlayer;
import mage.players.Player;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.stream.Collectors;

/**
 * A seeded random player for human seats (Rust migration: decision logs and shadow games). Answers each question as a
 * client would, from another thread, after the game thread opens the answer. What to answer is picked on the game
 * thread, where the question is asked, so it sees the game as the question does.
 */
public final class RandomDriver {
    private final Game game;
    private final ExecutorService answers;
    private final Random random;
    /** tries per priority window or payment, so a spell that can't be cast or paid is given up on */
    private final Map<String, Integer> tries = new HashMap<>();
    /** each question answered, in order: the query type and message, one per answer */
    private final List<String> questions = java.util.Collections.synchronizedList(new ArrayList<>());
    private String asking;

    public RandomDriver(Game game, ExecutorService answers, Random random) {
        this.game = game;
        this.answers = answers;
        this.random = random;
    }

    public void ask(PlayerQueryEvent event) {
        Player player = game.getPlayer(event.getPlayerId());
        if (!(player instanceof HumanPlayer)) {
            return;
        }
        HumanPlayer human = (HumanPlayer) player;
        String message = event.getMessage() == null ? "" : event.getMessage();
        asking = event.getQueryType() + " " + message;
        // a window ends when something changes: then every kind of answer gets its tries again
        String window = game.getTurnNum() + "/" + game.getTurnStepType() + "/" + game.getStack().size() + "/"
                + game.getBattlefield().count(mage.filter.StaticFilters.FILTER_PERMANENT, player.getId(), null, game) + "/"
                + player.getHand().size() + "/" + player.getId();
        switch (event.getQueryType()) {
            case ASK:
                answer(human, null, !message.contains("ulligan") && random.nextInt(3) > 0, null, null);
                break;
            case SELECT:
                if (message.startsWith("Play")) {
                    List<UUID> sources = new ArrayList<>();
                    List<UUID> lands = new ArrayList<>();
                    for (ActivatedAbility ability : player.getPlayable(game, true)) {
                        if (ability instanceof PlayLandAbility) {
                            lands.add(ability.getSourceId());
                        } else if (!ability.isManaAbility() && !sources.contains(ability.getSourceId())) {
                            sources.add(ability.getSourceId());
                        }
                    }
                    int tried = tries.merge("play " + window, 1, Integer::sum);
                    if (!lands.isEmpty() && tried == 1) {
                        answer(human, pick(lands), null, null, null);
                    } else if (!sources.isEmpty() && tried <= 3 && random.nextInt(10) < 8) {
                        answer(human, pick(sources), null, null, null);
                    } else {
                        answer(human, null, false, null, null);
                    }
                } else if (message.startsWith("Select attackers")) {
                    boolean none = game.getCombat().getAttackers().isEmpty();
                    if (none && random.nextInt(3) > 0) {
                        answer(human, null, null, null, "special");
                    } else {
                        answer(human, null, true, null, null);
                    }
                } else if (message.startsWith("Select blockers")) {
                    List<UUID> blockers = game.getBattlefield().getAllActivePermanents(player.getId()).stream()
                            .filter(p -> p.isCreature(game) && !p.isTapped() && p.getBlocking() == 0)
                            .map(Permanent::getId).collect(Collectors.toList());
                    if (!blockers.isEmpty() && tries.merge("block " + window, 1, Integer::sum) <= 1 && random.nextBoolean()) {
                        answer(human, pick(blockers), null, null, null);
                    } else {
                        answer(human, null, true, null, null);
                    }
                } else {
                    answer(human, null, true, null, null);
                }
                break;
            case PICK_TARGET: {
                List<UUID> candidates = new ArrayList<>();
                if (event.getTargets() != null) candidates.addAll(event.getTargets());
                if (event.getCards() != null) candidates.addAll(event.getCards());
                if (event.getPerms() != null) event.getPerms().forEach(p -> candidates.add(p.getId()));
                if (!candidates.isEmpty() && tries.merge("target " + window, 1, Integer::sum) <= 3) {
                    answer(human, pick(candidates), null, null, null);
                } else {
                    answer(human, null, true, null, null);
                }
                break;
            }
            case CHOOSE_ABILITY:
            case PICK_ABILITY:
                if (event.getAbilities() != null && !event.getAbilities().isEmpty()) {
                    answer(human, event.getAbilities().get(random.nextInt(event.getAbilities().size())).getId(), null, null, null);
                } else {
                    answer(human, null, false, null, null);
                }
                break;
            case CHOOSE_MODE:
                answer(human, pick(new ArrayList<>(event.getModes().keySet())), null, null, null);
                break;
            case CHOOSE_CHOICE: {
                List<String> choices = event.getChoice().isKeyChoice()
                        ? new ArrayList<>(event.getChoice().getKeyChoices().keySet())
                        : new ArrayList<>(event.getChoice().getChoices());
                choices.sort(null);
                answer(human, null, choices.isEmpty() ? Boolean.FALSE : null, null,
                        choices.isEmpty() ? null : choices.get(random.nextInt(choices.size())));
                break;
            }
            case PLAY_MANA: {
                List<UUID> sources = game.getBattlefield().getAllActivePermanents(player.getId()).stream()
                        .filter(p -> !p.isTapped() && !p.getAbilities().getActivatedManaAbilities(Zone.BATTLEFIELD).isEmpty())
                        .map(Permanent::getId).collect(Collectors.toList());
                if (!sources.isEmpty() && tries.merge("mana " + window, 1, Integer::sum) <= 8) {
                    answer(human, pick(sources), null, null, null);
                } else {
                    answer(human, null, false, null, null);
                }
                break;
            }
            case AMOUNT:
                answer(human, null, null, event.getMin() + random.nextInt(Math.max(1, Math.min(20, event.getMax() - event.getMin() + 1))), null);
                break;
            case MULTI_AMOUNT: {
                // the dialog's own defaults (combat damage: lethal to each blocker in order), as a client sends them
                List<Integer> values = mage.constants.MultiAmountType.prepareDefaultValues(event.getMessages(), event.getMin(), event.getMax());
                answer(human, null, null, null, values.stream().map(String::valueOf).collect(Collectors.joining(" ")));
                break;
            }
            case PLAY_X_MANA:
            case CHOOSE_PILE:
                answer(human, null, true, null, null);
                break;
            default:
                // messages, drafts and deck building: nothing to answer
        }
    }

    private UUID pick(List<UUID> ids) {
        return ids.get(random.nextInt(ids.size()));
    }

    public List<String> getQuestions() {
        synchronized (questions) {
            return new ArrayList<>(questions);
        }
    }

    private void answer(HumanPlayer player, UUID id, Boolean bool, Integer integer, String string) {
        questions.add(asking);
        answers.execute(() -> {
            if (id != null) {
                player.setResponseUUID(id);
            } else if (string != null) {
                player.setResponseString(string);
            } else if (integer != null) {
                player.setResponseInteger(integer);
            } else {
                player.setResponseBoolean(bool);
            }
        });
    }
}
