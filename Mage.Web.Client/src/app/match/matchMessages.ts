import { registerMessages } from '../i18n';
import messages from '../i18n/en/match';
import game from '../i18n/en/game';

// the match stage's English, registered when the first of its parts loads (with the game or the build screen)
registerMessages(messages);
registerMessages(game);
