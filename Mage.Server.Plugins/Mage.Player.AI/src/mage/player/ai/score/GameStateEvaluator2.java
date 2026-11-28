package mage.player.ai.score;

import mage.abilities.Ability;
import mage.abilities.effects.Effect;
import mage.constants.CommanderCardType;
import mage.constants.Outcome;
import mage.game.Game;
import mage.game.permanent.Permanent;
import mage.players.Player;
import mage.watchers.common.CommanderInfoWatcher;
import org.apache.log4j.Logger;

import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.UUID;

/**
 * Evaluates a game state for an AI player, supporting both 1v1 duels and multiplayer / Commander pods.
 */
public final class GameStateEvaluator2 {

    private static final Logger logger = Logger.getLogger(GameStateEvaluator2.class);

    public static final int WIN_GAME_SCORE = 100000000;
    public static final int LOSE_GAME_SCORE = -WIN_GAME_SCORE;

    public static final int HAND_CARD_SCORE = 5;
    private static final int COMMANDER_ON_BOARD_BONUS = 250;
    private static final int ELIMINATED_OPPONENT_BONUS = 1500;

    public static PlayerEvaluateScore evaluate(UUID playerId, Game game) {
        return evaluate(playerId, game, true);
    }

    public static PlayerEvaluateScore evaluate(UUID playerId, Game game, boolean useCombatPermanentScore) {
        Player player = game.getPlayer(playerId);
        if (player == null) {
            return new PlayerEvaluateScore(playerId, LOSE_GAME_SCORE);
        }

        Set<UUID> allOpponentIds = game.getOpponents(playerId, false);
        if (allOpponentIds.isEmpty()) {
            return new PlayerEvaluateScore(playerId, WIN_GAME_SCORE);
        }

        List<Player> livingOpponents = new ArrayList<>();
        int eliminatedOpponents = 0;
        for (UUID oppId : allOpponentIds) {
            Player opp = game.getPlayer(oppId);
            if (opp != null && opp.isInGame() && !opp.hasLost() && opp.getLife() > 0) {
                livingOpponents.add(opp);
            } else {
                eliminatedOpponents++;
            }
        }

        if (game.checkIfGameIsOver() || player.hasLost() || livingOpponents.isEmpty()) {
            if (player.hasLost()) {
                return new PlayerEvaluateScore(playerId, LOSE_GAME_SCORE);
            }
            for (UUID oppId : allOpponentIds) {
                Player opp = game.getPlayer(oppId);
                if (opp != null && opp.hasWon()) {
                    return new PlayerEvaluateScore(playerId, LOSE_GAME_SCORE);
                }
            }
            if (player.hasWon() || livingOpponents.isEmpty()) {
                return new PlayerEvaluateScore(playerId, WIN_GAME_SCORE);
            }
        }

        // Preserve exact 1v1 scoring when there is only one opponent in the match
        if (allOpponentIds.size() == 1) {
            Player opponent = livingOpponents.get(0);
            int playerLifeScore;
            int opponentLifeScore = 0;
            if (player.getLife() <= 0) {
                playerLifeScore = ArtificialScoringSystem.LOSE_GAME_SCORE;
            } else if (opponent.getLife() <= 0) {
                playerLifeScore = ArtificialScoringSystem.WIN_GAME_SCORE;
            } else {
                playerLifeScore = ArtificialScoringSystem.getLifeScore(player.getLife());
                opponentLifeScore = ArtificialScoringSystem.getLifeScore(opponent.getLife());
            }

            Set<UUID> playerCommanders = commanderIds(game, player);
            Set<UUID> oppCommanders = commanderIds(game, opponent);

            int playerPermanentsScore = scorePermanents(game, playerId, playerCommanders, useCombatPermanentScore);
            int opponentPermanentsScore = scorePermanents(game, opponent.getId(), oppCommanders, useCombatPermanentScore);

            playerLifeScore -= commanderDamagePenalty(game, oppCommanders, playerId);
            opponentLifeScore -= commanderDamagePenalty(game, playerCommanders, opponent.getId());

            int playerHandScore = player.getHand().size() * HAND_CARD_SCORE;
            int opponentHandScore = opponent.getHand().size() * HAND_CARD_SCORE;

            return new PlayerEvaluateScore(
                    playerId,
                    playerLifeScore, playerHandScore, playerPermanentsScore,
                    opponentLifeScore, opponentHandScore, opponentPermanentsScore);
        }

        // Multiplayer / Commander pod evaluation: aggregate across all living opponents,
        // weighting the biggest threat (archenemy) higher and rewarding eliminating opponents.
        int playerLifeScore = player.getLife() <= 0
                ? ArtificialScoringSystem.LOSE_GAME_SCORE
                : ArtificialScoringSystem.getLifeScore(player.getLife()) + eliminatedOpponents * ELIMINATED_OPPONENT_BONUS;
        Set<UUID> playerCommanders = commanderIds(game, player);
        int playerPermanentsScore = scorePermanents(game, playerId, playerCommanders, useCombatPermanentScore);
        int playerHandScore = player.getHand().size() * HAND_CARD_SCORE;

        int oppCount = livingOpponents.size();
        int[] oppLifeScores = new int[oppCount];
        int[] oppPermScores = new int[oppCount];
        int[] oppHandScores = new int[oppCount];
        int maxThreatIdx = 0;
        int maxThreatTotal = Integer.MIN_VALUE;

        for (int i = 0; i < oppCount; i++) {
            Player opp = livingOpponents.get(i);
            Set<UUID> oppCommanders = commanderIds(game, opp);
            oppLifeScores[i] = ArtificialScoringSystem.getLifeScore(opp.getLife())
                    - commanderDamagePenalty(game, playerCommanders, opp.getId());
            oppPermScores[i] = scorePermanents(game, opp.getId(), oppCommanders, useCombatPermanentScore);
            oppHandScores[i] = opp.getHand().size() * HAND_CARD_SCORE;
            playerLifeScore -= commanderDamagePenalty(game, oppCommanders, playerId);

            int total = oppLifeScores[i] + oppPermScores[i] + oppHandScores[i];
            if (total > maxThreatTotal) {
                maxThreatTotal = total;
                maxThreatIdx = i;
            }
        }

        // Weight the strongest opponent at 1.5x (3/2) and other opponents at 1.0x (2/2), normalized by total weight
        long weightedLife = 0;
        long weightedPerm = 0;
        long weightedHand = 0;
        long totalWeight = 0;
        for (int i = 0; i < oppCount; i++) {
            int weight = (i == maxThreatIdx && oppCount > 1) ? 3 : 2;
            weightedLife += (long) oppLifeScores[i] * weight;
            weightedPerm += (long) oppPermScores[i] * weight;
            weightedHand += (long) oppHandScores[i] * weight;
            totalWeight += weight;
        }

        int opponentLifeScore = (int) (weightedLife / totalWeight);
        int opponentPermanentsScore = (int) (weightedPerm / totalWeight);
        int opponentHandScore = (int) (weightedHand / totalWeight);

        return new PlayerEvaluateScore(
                playerId,
                playerLifeScore, playerHandScore, playerPermanentsScore,
                opponentLifeScore, opponentHandScore, opponentPermanentsScore);
    }

    private static Set<UUID> commanderIds(Game game, Player player) {
        try {
            return game.getCommandersIds(player, CommanderCardType.COMMANDER_OR_OATHBREAKER, false);
        } catch (Throwable t) {
            return java.util.Collections.emptySet();
        }
    }

    private static int scorePermanents(Game game, UUID controllerId, Set<UUID> commanders, boolean useCombatPermanentScore) {
        int total = 0;
        try {
            for (Permanent permanent : game.getBattlefield().getAllActivePermanents(controllerId)) {
                int onePermScore = evaluatePermanent(permanent, game, useCombatPermanentScore);
                if (!commanders.isEmpty() && commanders.contains(permanent.getId())) {
                    onePermScore += COMMANDER_ON_BOARD_BONUS;
                }
                total += onePermScore;
            }
        } catch (Throwable t) {
            // ignore transient simulation state errors
        }
        return total;
    }

    private static int commanderDamagePenalty(Game game, Set<UUID> attackingCommanders, UUID victimId) {
        if (attackingCommanders.isEmpty()) {
            return 0;
        }
        int penalty = 0;
        for (UUID cmdId : attackingCommanders) {
            CommanderInfoWatcher watcher = game.getState().getWatcher(CommanderInfoWatcher.class, cmdId);
            if (watcher == null) {
                continue;
            }
            int dealt = watcher.getDamageToPlayer().getOrDefault(victimId, 0);
            if (dealt >= 21) {
                penalty += 50000;
            } else if (dealt > 0) {
                // Escalating pressure as commander damage nears 21
                penalty += dealt * 15 + (dealt >= 14 ? (dealt - 13) * 60 : 0);
            }
        }
        return penalty;
    }

    public static int evaluatePermanent(Permanent permanent, Game game, boolean useCombatPermanentScore) {
        // prevent AI from attaching bad auras to its own permanents ex: Brainwash and Demonic Torment (no immediate penalty on the battlefield)
        int value = 0;
        if (!permanent.getAttachments().isEmpty()) {
            for (UUID attachmentId : permanent.getAttachments()) {
                Permanent attachment = game.getPermanent(attachmentId);
                for (Ability a : attachment.getAbilities(game)) {
                    for (Effect e : a.getEffects()) {
                        if (e.getOutcome().equals(Outcome.Detriment)
                                && attachment.getControllerId().equals(permanent.getControllerId())) {
                            value -= 1000;  // seems to work well ; -300 is not effective enough
                        }
                    }
                }
            }
        }
        value += ArtificialScoringSystem.getFixedPermanentScore(game, permanent);
        value += ArtificialScoringSystem.getDynamicPermanentScore(game, permanent);
        if (useCombatPermanentScore) {
            value += ArtificialScoringSystem.getCombatPermanentScore(game, permanent);
        }
        return value;
    }

    public static class PlayerEvaluateScore {

        private UUID playerId;
        private int playerLifeScore = 0;
        private int playerHandScore = 0;
        private int playerPermanentsScore = 0;

        private int opponentLifeScore = 0;
        private int opponentHandScore = 0;
        private int opponentPermanentsScore = 0;

        private int specialScore = 0; // special score (ignore all others, e.g. for win/lose game states)

        public PlayerEvaluateScore(UUID playerId, int specialScore) {
            this.playerId = playerId;
            this.specialScore = specialScore;
        }

        public PlayerEvaluateScore(UUID playerId,
                                   int playerLifeScore, int playerHandScore, int playerPermanentsScore,
                                   int opponentLifeScore, int opponentHandScore, int opponentPermanentsScore) {
            this.playerId = playerId;
            this.playerLifeScore = playerLifeScore;
            this.playerHandScore = playerHandScore;
            this.playerPermanentsScore = playerPermanentsScore;
            this.opponentLifeScore = opponentLifeScore;
            this.opponentHandScore = opponentHandScore;
            this.opponentPermanentsScore = opponentPermanentsScore;
        }

        public UUID getPlayerId() {
            return this.playerId;
        }

        public int getPlayerScore() {
            return playerLifeScore + playerHandScore + playerPermanentsScore;
        }

        public int getOpponentScore() {
            return opponentLifeScore + opponentHandScore + opponentPermanentsScore;
        }

        public int getTotalScore() {
            if (specialScore != 0) {
                return specialScore;
            } else {
                return getPlayerScore() - getOpponentScore();
            }
        }

        public int getPlayerLifeScore() {
            return playerLifeScore;
        }

        public int getPlayerHandScore() {
            return playerHandScore;
        }

        public int getPlayerPermanentsScore() {
            return playerPermanentsScore;
        }

        public String getPlayerInfoFull() {
            return "Life:" + playerLifeScore
                    + ", Hand:" + playerHandScore
                    + ", Perm:" + playerPermanentsScore;
        }

        public String getPlayerInfoShort() {
            return "L:" + playerLifeScore
                    + ",H:" + playerHandScore
                    + ",P:" + playerPermanentsScore;
        }

        public String getOpponentInfoFull() {
            return "Life:" + opponentLifeScore
                    + ", Hand:" + opponentHandScore
                    + ", Perm:" + opponentPermanentsScore;
        }

        public String getOpponentInfoShort() {
            return "L:" + opponentLifeScore
                    + ",H:" + opponentHandScore
                    + ",P:" + opponentPermanentsScore;
        }
    }
}
