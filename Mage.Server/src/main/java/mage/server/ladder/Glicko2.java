package mage.server.ladder;

import java.util.List;

/**
 * Glicko-2 (Glickman, "Example of the Glicko-2 system", 2013) for the ladder (#430): players and bots each have a
 * rating, a rating deviation (how sure the rating is) and a volatility (how erratic their results are). Ratings are on
 * the familiar Elo-like scale, the same one the bots' calibration (ladder-calibrate) measures.
 */
public final class Glicko2 {

    /** the conversion between the Glicko and Glicko-2 scales */
    static final double SCALE = 173.7178;
    /** the system constant: how much the volatility may change in a period */
    public static final double TAU = 0.5;
    /** a new player's volatility */
    public static final double START_VOLATILITY = 0.06;
    /** the widest deviation: nothing known */
    public static final double MAX_RD = 350;

    private static final double EPSILON = 0.000001;

    private Glicko2() {
    }

    /** A rating, its deviation and volatility. Immutable. */
    public static final class Rating {
        public final double rating;
        public final double rd;
        public final double volatility;

        public Rating(double rating, double rd, double volatility) {
            this.rating = rating;
            this.rd = rd;
            this.volatility = volatility;
        }

        @Override
        public String toString() {
            return String.format(java.util.Locale.ENGLISH, "%.1f ± %.1f (σ %.5f)", rating, rd, volatility);
        }
    }

    /** A game against an opponent: their rating and the score (1 a win, 0.5 a draw, 0 a loss) */
    public static final class Result {
        public final Rating opponent;
        public final double score;

        public Result(Rating opponent, double score) {
            this.opponent = opponent;
            this.score = score;
        }
    }

    /** The chance that `player` beats `opponent`, from both ratings and deviations */
    public static double expected(Rating player, Rating opponent) {
        double mu = (player.rating - 1500) / SCALE;
        double muJ = (opponent.rating - 1500) / SCALE;
        double phiJ = opponent.rd / SCALE;
        return e(mu, muJ, phiJ);
    }

    /**
     * The rating after one rating period's results. With no results only the deviation grows, by the volatility. The
     * new deviation is kept between `minRd` and {@link #MAX_RD}.
     */
    public static Rating update(Rating player, List<Result> results, double minRd) {
        double mu = (player.rating - 1500) / SCALE;
        double phi = player.rd / SCALE;
        double sigma = player.volatility;
        if (results.isEmpty()) {
            double grown = Math.sqrt(phi * phi + sigma * sigma) * SCALE;
            return new Rating(player.rating, clamp(grown, minRd), sigma);
        }
        double vInverse = 0;
        double sum = 0;
        for (Result result : results) {
            double muJ = (result.opponent.rating - 1500) / SCALE;
            double phiJ = result.opponent.rd / SCALE;
            double g = g(phiJ);
            double e = e(mu, muJ, phiJ);
            vInverse += g * g * e * (1 - e);
            sum += g * (result.score - e);
        }
        double v = 1 / vInverse;
        double delta = v * sum;

        // the new volatility, by the Illinois algorithm (step 5)
        double a = Math.log(sigma * sigma);
        double bigA = a;
        double bigB;
        if (delta * delta > phi * phi + v) {
            bigB = Math.log(delta * delta - phi * phi - v);
        } else {
            int k = 1;
            while (f(a - k * TAU, delta, phi, v, a) < 0) {
                k++;
            }
            bigB = a - k * TAU;
        }
        double fA = f(bigA, delta, phi, v, a);
        double fB = f(bigB, delta, phi, v, a);
        while (Math.abs(bigB - bigA) > EPSILON) {
            double bigC = bigA + (bigA - bigB) * fA / (fB - fA);
            double fC = f(bigC, delta, phi, v, a);
            if (fC * fB <= 0) {
                bigA = bigB;
                fA = fB;
            } else {
                fA = fA / 2;
            }
            bigB = bigC;
            fB = fC;
        }
        double newSigma = Math.exp(bigA / 2);

        double phiStar = Math.sqrt(phi * phi + newSigma * newSigma);
        double newPhi = 1 / Math.sqrt(1 / (phiStar * phiStar) + 1 / v);
        double newMu = mu + newPhi * newPhi * sum;
        return new Rating(newMu * SCALE + 1500, clamp(newPhi * SCALE, minRd), newSigma);
    }

    private static double clamp(double rd, double minRd) {
        return Math.max(minRd, Math.min(MAX_RD, rd));
    }

    private static double g(double phi) {
        return 1 / Math.sqrt(1 + 3 * phi * phi / (Math.PI * Math.PI));
    }

    private static double e(double mu, double muJ, double phiJ) {
        return 1 / (1 + Math.exp(-g(phiJ) * (mu - muJ)));
    }

    private static double f(double x, double delta, double phi, double v, double a) {
        double ex = Math.exp(x);
        double d = phi * phi + v + ex;
        return ex * (delta * delta - phi * phi - v - ex) / (2 * d * d) - (x - a) / (TAU * TAU);
    }
}
