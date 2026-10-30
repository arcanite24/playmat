package mage.game;

/**
 * An AI simulation stopped because its search was interrupted (the search's time limit). Thrown from the engine's
 * hot paths so a runaway simulation ends instead of holding a shared search thread.
 */
public class SimulationInterruptedException extends RuntimeException {

    public SimulationInterruptedException() {
        super("AI simulation interrupted", null, false, false);
    }
}
