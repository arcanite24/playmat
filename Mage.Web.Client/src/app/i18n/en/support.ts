/**
 * English for supporting the server: the month's goal bar (Home, sign-in), the /support page, the thank-you once a
 * payment reaches an account, and the supporter settings. It loads with them. Support is a donation, never a purchase:
 * say "support" and "thank you", never "buy".
 */
export default {
  // the goal bar; {month} is the month's name ("October 2026")
  'support.bar': 'Server costs for {month}',
  // {amount} and {goal} are dollar amounts
  'support.bar.amount': '{amount} of {goal}',
  'support.bar.left': '{amount} to go',
  'support.bar.reached': 'Covered. Thank you!',
  // a surplus is never hidden: it pays for busier months
  'support.bar.ahead': 'Ahead by {amount}, a buffer for busy months',
  'support.bar.supporters': { one: '{count} supporter this month', other: '{count} supporters this month' },
  'support.bar.progress': 'This month: {amount} of {goal}',
  'support.cta': 'Support the server',
  'support.cta.sponsors': 'GitHub Sponsors',
  'support.cta.newTab': 'opens in a new tab',
  'support.more': 'What it pays for',
  // {code} is the player's support code, e.g. PM-7Q4K
  'support.code': 'Your support code: {code}',
  'support.code.hint': 'Pay with your confirmed account email, or put this code in your message, and the thank-you finds your account by itself.',
  'support.code.copy': 'Copy code',
  'support.code.copied': 'Copied',

  // the /support page
  'support.title': 'Support the server',
  'support.lede': '{app} is free to play, always. Players who can chip in keep the server running for everyone, and get a small thank-you.',
  'support.costs': 'What it pays for',
  'support.costs.month': 'The goal is the month’s costs (calendar month, UTC, in US dollars). Anything above it is kept for the next busy month.',
  'support.perks': 'A thank-you, kept for good',
  'support.perks.text': 'Once your support reaches your account, you get the thank-you set:',
  'support.perk.sleeve': 'The supporter sleeve, with its own card back',
  'support.perk.playmat': 'The Patron playmat cloth',
  'support.perk.frame': 'A brass ring around your avatar',
  'support.perk.title': 'The Patron title under your name',
  'support.perk.flair': 'A small heart by your name in the lobby, chat and games',
  'support.perk.pet': 'The party corgi, a pet that sits by your side of the battlefield and celebrates when clicked',
  'support.corgi.alt': 'The party corgi, spinning in a starfield in party hat and sunglasses, beside a blinking NEW! and rainbow letters: THANK U 4 SUPPORTING!!',
  // a joke: the GIF is drawn like a 1997 homepage
  'support.corgi.caption': 'Best viewed in Netscape Navigator at 800×600',
  'support.perks.status': 'While a membership or sponsorship runs, and for 30 days after a donation, you also show as a supporter. The thank-you set stays either way.',
  'support.how': 'How it reaches your account',
  'support.how.text': 'Pay with your account’s email address (once it is confirmed), or put your support code in the Ko-fi message. GitHub Sponsors are matched through Sign in with GitHub. It takes about a minute; if it doesn’t, the staff link it by hand.',
  'support.how.signIn': 'Sign in to see your support code.',
  'support.wall': 'Thank you',
  'support.wall.empty': 'Nobody yet this month. You could be the first.',
  'support.wall.note': 'Public Ko-fi names show here, and account names only for players who choose it in Settings.',
  'support.rules': 'The rules',
  'support.rule.free': 'Free forever: every format, card, queue, Career feature and game is open to everyone, supporter or not.',
  'support.rule.cosmetic': 'The thank-you is cosmetic only, drawn in {app}’s own style, never Wizards of the Coast art.',
  'support.rule.voluntary': 'Support is a voluntary donation, not a purchase. Wizards of the Coast’s Fan Content Policy allows donations only when they change nobody’s access, and they don’t.',
  'support.rule.kept': 'The thank-you is yours for good. The one exception: if a payment is refunded or charged back, the staff may take back what it gave.',
  'support.rule.refunds': 'Refunds follow Ko-fi’s process (or GitHub’s, for GitHub Sponsors).',
  'support.off': 'This server isn’t asking for support right now.',
  'support.unreachable': 'Couldn’t reach the server. Try again in a moment.',
  'support.back': 'Back',
  'support.backTo': 'Back to {app}',

  // the thank-you, once, on the next visit after a payment reaches the account
  'support.notice.title': 'Thank you!',
  'support.notice.text': 'Your support reached your account. Your supporter sleeve, playmat, frame, title, flair and party corgi are ready.',
  'support.notice.where': 'Choose the sleeve for a deck in Decks and the cloth in Settings, Playmat. The ring, the title and the heart show by your name, and the corgi comes to every game.',
  'support.notice.close': 'Lovely',

  // a thank-you item the player doesn't have yet, shown quietly in the pickers
  'support.locked': '{name}: a thank-you for supporting the server',
  'support.sleeve': 'Supporter sleeve',

  // Settings, Account
  'support.settings': 'Supporting the server',
  'support.settings.thanks': 'Thank you for supporting the server.',
  'support.settings.active': 'Thank you! You’re a supporter this month.',
  'support.settings.none': 'Supporting the server gives you a thank-you set: a sleeve, a playmat, a ring, a title, a heart and a pet corgi.',
  'support.settings.wall': 'Show my name on the supporters wall',
  'support.settings.wall.detail': 'Off until you choose it. Your account name shows on the support page in the months you support.',
  'support.settings.wear': 'Choose the ring, the title, the heart and the pet you wear in Settings, Cosmetics.',

  // pets on the battlefield; {name} is the player's name
  'pet.party-corgi': 'Party corgi',
  'pet.label': '{pet}, {name}’s pet. Click to celebrate.',
  'pet.mine': '{pet}, your pet. Click to celebrate.',

  // the thank-you set in Career's locker (Progress, Look): everyone sees it, a non-supporter's is locked
  'support.locker.locked': '{name}: supporters only. Opens the support page.',
  'support.locker.only': 'Supporters only',
  'support.locker.patron': 'Patron',
  'support.locker.pets': 'Pets',
  'support.locker.petsNote': 'A pet sits by your side of the battlefield in every game, and everyone at the table can click it.',
  'support.locker.noPet': 'No pet',
  'support.locker.marks': 'Ring and heart',
  'support.locker.marksNote': 'The ring goes round your avatar and the heart by your name, wherever others see you.',
  'support.locker.ring': 'Supporter ring',
  'support.locker.heart': 'Heart by your name',
  'support.locker.patronMat': 'Patron playmat',
} as const;
