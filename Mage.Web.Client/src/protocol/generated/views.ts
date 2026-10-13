/* tslint:disable */
/* eslint-disable */

export interface AbilityPickerView extends Serializable {
    choices?: { [index: string]: string };
    gameView?: GameView;
    message?: string;
}

export interface AccessOverview {
    accounts?: boolean;
    allowlist?: AllowEntry[];
    github?: boolean;
    githubPeople?: number;
    githubProblem?: string;
    githubRepo?: string;
    githubSyncedAt?: number;
    invites?: Invite[];
    mail?: boolean;
    mode?: string;
    notice?: string;
    owner?: string;
    requests?: AccessRequest[];
}

export interface AccessRequest {
    createdAt?: number;
    decidedAt?: number;
    decidedBy?: string;
    email?: string;
    id?: number;
    name?: string;
    note?: string;
    status?: string;
}

export interface AccountDetail {
    account?: AccountRow;
    audit?: AuditEntry[];
    decks?: number;
    feedback?: FeedbackEntry[];
    githubLogin?: string;
    matches?: number;
    matchesQuit?: number;
    rating?: number;
    reports?: PlayerReport[];
    sessions?: AccountSession[];
    tourneys?: number;
}

export interface AccountRow {
    active?: boolean;
    allowlisted?: boolean;
    career?: boolean;
    email?: string;
    lastConnection?: number;
    lockedUntil?: number;
    mutedUntil?: number;
    name?: string;
    online?: boolean;
    staffRole?: string;
}

export interface AccountSession {
    connectedAt?: number;
    host?: string;
    lastActivity?: number;
    playing?: string;
    sessionId?: string;
    state?: string;
}

export interface AccountsPage {
    accounts?: AccountRow[];
    offset?: number;
    total?: number;
}

export interface Advice extends Serializable {
    kind?: AdviceKind;
    millis?: number;
    steps?: AdviceStep[];
    text?: string;
    when?: string;
}

export interface AdviceStep extends Serializable {
    objectId?: string;
    targetIds?: string[];
    text?: string;
}

export interface AllowEntry {
    addedAt?: number;
    addedBy?: string;
    email?: string;
    id?: number;
    name?: string;
    note?: string;
}

export interface AuditEntry {
    action?: string;
    actor?: string;
    at?: number;
    detail?: string;
    id?: number;
    reason?: string;
    target?: string;
}

export interface BackupRun {
    at?: number;
    lastOkAt?: number;
    name?: string;
    ok?: boolean;
    problem?: string;
    size?: string;
}

export interface BasicLandSetInfo extends ExpansionSetInfo {
    hasSnowBasics?: boolean;
}

export interface BatchJob {
    created?: number;
    done?: number;
    error?: string;
    finished?: number;
    games?: number;
    id?: string;
    kind?: string;
    maxTurns?: number;
    nameA?: string;
    nameB?: string;
    result?: JobResult;
    seed?: number;
    skillA?: number;
    skillB?: number;
    started?: number;
    status?: string;
    user?: string;
}

export interface BatchStatus {
    concurrency?: number;
    gamesInFlight?: number;
    gamesPerDay?: number;
    jobs?: BatchJob[];
}

export interface BracketGroupInfo {
    cards?: string[];
    count?: number;
    max?: number;
    name?: string;
}

export interface BracketInfo {
    badCards?: string[];
    groups?: BracketGroupInfo[];
    level?: number;
    name?: string;
    shortName?: string;
    valid?: boolean;
}

export interface BridgeSnapshot {
    bucketLimits?: number[];
    buckets?: number[];
    chars?: number;
    connections?: number;
    heaviest?: CallbackStats[];
    large?: LargeMessage[];
    messages?: number;
    requests?: number;
    startedAt?: number;
    stateChars?: number;
    stateComplete?: number;
    stateFullChars?: number;
    statePatches?: number;
    stateResyncs?: number;
}

export interface CallbackStats {
    chars?: number;
    count?: number;
    maxChars?: number;
    method?: string;
}

export interface CardCriteria {
    black?: boolean;
    blue?: boolean;
    colorMatch?: string;
    colorless?: boolean;
    count?: number;
    doubleFaced?: boolean;
    excludedRarities?: Rarity[];
    format?: string;
    green?: boolean;
    ignoreSetCodes?: string[];
    manaValue?: number;
    manaValueOperator?: string;
    maxCardNumber?: number;
    minCardNumber?: number;
    modalDoubleFaced?: boolean;
    name?: string;
    nameContains?: string;
    nightCard?: boolean;
    notSupertypes?: SuperType[];
    notTypes?: CardType[];
    rarities?: Rarity[];
    red?: boolean;
    rules?: string;
    searchNames?: boolean;
    searchRules?: boolean;
    searchText?: string;
    searchTypes?: boolean;
    setCodes?: string[];
    sortBy?: string;
    start?: number;
    subtypes?: SubType[];
    supertypes?: SuperType[];
    types?: CardType[];
    uniqueNames?: boolean;
    variousArt?: boolean;
    webColors?: string[];
    webExcludedColors?: string[];
    white?: boolean;
}

export interface CardIcon extends Copyable<CardIcon> {
}

export interface CardView extends SimpleCardView {
    ability?: CardView;
    abilityType?: AbilityType;
    alternateName?: string;
    artRect?: ArtRect;
    bandedCards?: string[];
    canAttack?: boolean;
    canBlock?: boolean;
    cardIcons?: CardIcon[];
    cardId?: string;
    cardTypes?: CardType[];
    color?: ObjectColor;
    controlledByOwner?: boolean;
    controllerId?: string;
    counters?: CounterView[];
    defense?: string;
    displayFullName?: string;
    displayName?: string;
    extraDeckCard?: boolean;
    faceDown?: boolean;
    flipCard?: boolean;
    frameColor?: ObjectColor;
    frameStyle?: FrameStyle;
    hideInfo?: boolean;
    imageFileName?: string;
    imageNumber?: number;
    inViewerOnly?: boolean;
    isAbility?: boolean;
    isDoubleFacedCard?: boolean;
    isSplitCard?: boolean;
    isToken?: boolean;
    leftSplitCostsStr?: string;
    leftSplitName?: string;
    leftSplitRules?: string[];
    leftSplitTypeLine?: string;
    loyalty?: string;
    mageObjectType?: MageObjectType;
    manaCostLeftStr?: string[];
    manaCostRightStr?: string[];
    manaValue?: number;
    name?: string;
    originalColorIdentity?: string;
    originalIsCopy?: boolean;
    originalPower?: MageInt;
    originalToughness?: MageInt;
    ownerId?: string;
    paid?: boolean;
    pairedCard?: string;
    parentId?: string;
    power?: string;
    rarity?: Rarity;
    rightSplitCostsStr?: string;
    rightSplitName?: string;
    rightSplitRules?: string[];
    rightSplitSpellType?: string;
    rightSplitTypeLine?: string;
    rotate?: boolean;
    rules?: string[];
    secondCardFace?: CardView;
    startingDefense?: string;
    startingLoyalty?: string;
    subTypes?: SubType[];
    superTypes?: SuperType[];
    targets?: string[];
    toughness?: string;
    transformable?: boolean;
    transformed?: boolean;
    zone?: Zone;
}

export interface CareerAchievement {
    achieved?: boolean;
    at?: number;
    coins?: number;
    cosmetic?: CareerCosmetic;
    id?: string;
    name?: string;
    progress?: number;
    scope?: string;
    secret?: boolean;
    target?: number;
    text?: string;
    xp?: number;
}

export interface CareerCampaign {
    bosses?: number;
    chapters?: CareerChapter[];
    colors?: string;
    deckType?: string;
    done?: number;
    format?: string;
    gameType?: string;
    id?: string;
    missing?: string[];
    name?: string;
    open?: boolean;
    order?: number;
    prologue?: string;
    requires?: string[];
    school?: boolean;
    summary?: string;
    text?: string;
    total?: number;
}

export interface CareerCard {
    cardNumber?: string;
    convertedTo?: string;
    count?: number;
    name?: string;
    rarity?: string;
    setCode?: string;
}

export interface CareerChallenge {
    attempts?: number;
    bestLife?: number;
    bestTurns?: number;
    board?: CareerChallengeEntry[];
    cover?: CareerCover;
    id?: string;
    name?: string;
    opponentName?: string;
    skill?: number;
    text?: string;
    twists?: CareerTwists;
    week?: string;
    won?: boolean;
}

export interface CareerChallengeEntry {
    attempts?: number;
    life?: number;
    rank?: number;
    turns?: number;
    user?: string;
    won?: boolean;
    you?: boolean;
}

export interface CareerChapter {
    color?: string;
    deckName?: string;
    done?: boolean;
    id?: string;
    name?: string;
    nodes?: CareerNode[];
    text?: string;
    unlockSet?: string;
}

export interface CareerCosmetic {
    id?: string;
    kind?: string;
}

export interface CareerCover {
    cardNumber?: string;
    name?: string;
    setCode?: string;
}

export interface CareerGameResult {
    achievements?: CareerAchievement[];
    career?: boolean;
    coins?: number;
    gameKey?: string;
    levelAfter?: number;
    levelBefore?: number;
    levelRewards?: CareerLevelReward[];
    mode?: CareerModeResult;
    note?: string;
    opened?: string[];
    opponent?: string;
    packs?: number;
    quests?: CareerQuestProgress[];
    weekly?: CareerWeekly;
    won?: boolean;
    xp?: number;
    xpForNext?: number;
    xpIntoLevel?: number;
}

export interface CareerGauntletState {
    bestWins?: number;
    entryCoins?: number;
    maxWins?: number;
    recent?: CareerRun[];
    rewards?: CareerRunReward[];
    run?: CareerRun;
}

export interface CareerLesson {
    cards?: string[];
    text?: string;
    title?: string;
}

export interface CareerLevelReward {
    coins?: number;
    cosmetic?: CareerCosmetic;
    level?: number;
    packs?: number;
    wildcard?: string;
}

export interface CareerLevelTrack {
    level?: number;
    rewards?: CareerLevelReward[];
    unlocks?: CareerCosmetic[];
    xp?: number;
    xpForNext?: number;
    xpIntoLevel?: number;
}

export interface CareerLimitedRun {
    deck?: DeckCardInfo[];
    id?: string;
    kind?: string;
    losses?: number;
    nextReward?: number;
    pack?: CareerPoolCard[];
    phase?: string;
    pick?: number;
    pool?: CareerPoolCard[];
    round?: number;
    setCode?: string;
    setName?: string;
    state?: string;
    wins?: number;
}

export interface CareerLimitedState {
    draftEntry?: number;
    draftRewards?: number[];
    recent?: CareerLimitedRun[];
    run?: CareerLimitedRun;
    sealedEntry?: number;
    sealedRewards?: number[];
}

export interface CareerLines {
    bigPlay?: string;
    intro?: string;
    lose?: string;
    lowLife?: string;
    win?: string;
}

export interface CareerMatch {
    opponentId?: string;
    tableId?: string;
    warm?: CareerCover[];
}

export interface CareerModeResult {
    coins?: number;
    cosmetics?: CareerCosmetic[];
    kind?: string;
    lines?: string[];
    ref?: string;
    stars?: number;
    title?: string;
    trial?: boolean;
    unlockedSet?: string;
    xp?: number;
}

export interface CareerNode {
    after?: string;
    before?: string;
    boss?: boolean;
    choice?: string;
    cover?: CareerCover;
    deckName?: string;
    gameType?: string;
    hint?: string;
    id?: string;
    lesson?: CareerLesson;
    name?: string;
    objective?: string;
    opponentName?: string;
    options?: CareerOption[];
    others?: CareerSeat[];
    requires?: string[];
    reward?: CareerReward;
    sideboard?: string;
    skill?: number;
    state?: string;
    text?: string;
    tips?: CareerTip[];
    twists?: CareerTwists;
    type?: string;
    winsNeeded?: number;
}

export interface CareerOffer {
    kind?: string;
    options?: CareerOfferOption[];
    pick?: number;
}

export interface CareerOfferOption {
    cards?: CareerRunCard[];
    color?: string;
    id?: string;
    name?: string;
}

export interface CareerOpponent {
    colors?: string;
    cover?: CareerCover;
    id?: string;
    lines?: CareerLines;
    name?: string;
    tagline?: string;
    tier?: number;
    tierName?: string;
    unlocked?: boolean;
    winCoins?: number;
    wins?: number;
}

export interface CareerOption {
    id?: string;
    reward?: CareerReward;
    text?: string;
}

export interface CareerPackResult {
    cards?: CareerCard[];
    profile?: CareerProfile;
    setCode?: string;
}

export interface CareerPayout {
    at?: number;
    coins?: number;
    matchKey?: string;
    note?: string;
    opponent?: string;
    won?: boolean;
    xp?: number;
}

export interface CareerPoolCard {
    cardNumber?: string;
    colors?: string;
    creature?: boolean;
    land?: boolean;
    manaValue?: number;
    name?: string;
    rarity?: string;
    setCode?: string;
}

export interface CareerProfile {
    cards?: number;
    coins?: number;
    countAiGames?: boolean;
    created?: number;
    level?: number;
    losses?: number;
    packTokens?: number;
    starter?: string;
    wildcards?: Wildcards;
    wins?: number;
    xp?: number;
}

export interface CareerPuzzle {
    attempts?: number;
    hint?: string;
    id?: string;
    name?: string;
    number?: number;
    open?: boolean;
    opponent?: CareerPuzzleSide;
    solved?: boolean;
    stars?: number;
    text?: string;
    you?: CareerPuzzleSide;
}

export interface CareerPuzzleSide extends Serializable {
    battlefield?: string[];
    graveyard?: string[];
    hand?: string[];
    library?: string[];
    life?: number;
}

export interface CareerQuest {
    coins?: number;
    id?: string;
    progress?: number;
    slot?: number;
    target?: number;
    text?: string;
    xp?: number;
}

export interface CareerQuestProgress {
    after?: number;
    before?: number;
    coins?: number;
    completed?: boolean;
    id?: string;
    target?: number;
    text?: string;
    xp?: number;
}

export interface CareerQuests {
    canReroll?: boolean;
    quests?: CareerQuest[];
    today?: string;
}

export interface CareerReward {
    coins?: number;
    cosmetics?: CareerCosmetic[];
    deckCards?: boolean;
    packs?: number;
    unlockSet?: string;
    wildcard?: string;
    xp?: number;
}

export interface CareerRun {
    bosses?: CareerRunBoss[];
    colors?: string;
    deck?: CareerRunCard[];
    deckSize?: number;
    ended?: number;
    id?: string;
    losses?: number;
    maxWins?: number;
    next?: CareerRunBoss;
    offer?: CareerOffer;
    started?: number;
    state?: string;
    wins?: number;
}

export interface CareerRunBoss {
    beaten?: boolean;
    cover?: CareerCover;
    name?: string;
    number?: number;
    skill?: number;
    twists?: CareerTwists;
}

export interface CareerRunCard {
    amount?: number;
    cardNumber?: string;
    name?: string;
    setCode?: string;
}

export interface CareerRunReward {
    coins?: number;
    packs?: number;
    wildcard?: string;
    wins?: number;
}

export interface CareerSeat {
    cover?: CareerCover;
    name?: string;
    skill?: number;
}

export interface CareerSetProgress {
    name?: string;
    owned?: number;
    setCode?: string;
    total?: number;
}

export interface CareerShopSet {
    cover?: CareerCover;
    locked?: boolean;
    name?: string;
    price?: number;
    releaseDate?: string;
    setCode?: string;
    unlockedBy?: string;
}

export interface CareerStarter {
    cards?: number;
    colors?: string;
    cover?: CareerCover;
    id?: string;
    name?: string;
}

export interface CareerState {
    enabled?: boolean;
    profile?: CareerProfile;
    recent?: CareerPayout[];
}

export interface CareerTip {
    on?: string;
    text?: string;
    title?: string;
}

export interface CareerTwists extends Serializable {
    emblem?: string;
    extraCards?: number;
    permanents?: string[];
    startingLife?: number;
    timed?: TimedTwist[];
}

export interface CareerWeekly {
    goals?: number[];
    nextGoal?: number;
    week?: string;
    wins?: number;
}

export interface ChatMessage extends Serializable {
    color?: MessageColor;
    message?: string;
    messageType?: MessageType;
    soundToPlay?: SoundToPlay;
    time?: DateAsNumber;
    turnInfo?: string;
    username?: string;
}

export interface Choice extends Serializable, Copyable<Choice> {
}

export interface ChoiceImpl extends Choice {
    choice?: string;
    choiceKey?: string;
    choices?: string[];
    chosenNormal?: boolean;
    chosenSpecial?: boolean;
    hintData?: { [index: string]: string[] };
    hintType?: ChoiceHintType;
    keyChoices?: { [index: string]: string };
    manaColorChoice?: boolean;
    message?: string;
    required?: boolean;
    searchEnabled?: boolean;
    searchText?: string;
    sortData?: { [index: string]: number };
    specialCanBeEmpty?: boolean;
    specialEnabled?: boolean;
    specialHint?: string;
    specialText?: string;
    subMessage?: string;
}

export interface ClientError {
    appVersion?: string;
    context?: string;
    message?: string;
    path?: string;
    source?: string;
    stack?: string;
    userAgent?: string;
}

export interface ClientErrorGroup {
    appVersion?: string;
    context?: string;
    count?: number;
    fingerprint?: string;
    firstSeen?: number;
    id?: number;
    lastSeen?: number;
    lastUser?: string;
    message?: string;
    path?: string;
    regressed?: boolean;
    resolved?: boolean;
    resolvedAt?: number;
    resolvedBy?: string;
    source?: string;
    stack?: string;
    userAgent?: string;
}

export interface CombatGroupView extends Serializable {
    attackers?: { [index: string]: CardView };
    blockers?: { [index: string]: CardView };
    defenderId?: string;
    defenderName?: string;
    isBlocked?: boolean;
}

export interface CommandObjectView extends SelectableObjectView {
}

export interface CommanderView extends CardView, CommandObjectView, Serializable {
}

export interface Comparable<T> {
}

export interface ContentPackView {
    author?: string;
    campaigns?: number;
    challenges?: number;
    decks?: number;
    id?: string;
    loaded?: boolean;
    name?: string;
    opponents?: number;
    problems?: string[];
    puzzles?: number;
    version?: string;
}

export interface Copyable<T> {
}

export interface CounterView extends Serializable {
    count?: number;
    name?: string;
}

export interface DeckCardInfo extends Serializable, Copyable<DeckCardInfo> {
    amount?: number;
    cardName?: string;
    cardNumber?: string;
    setCode?: string;
}

export interface DeckCardLayout extends Copyable<DeckCardLayout> {
    cards?: DeckCardInfo[][][];
    settings?: string;
}

export interface DeckCardLists extends Serializable, Copyable<DeckCardLists> {
    author?: string;
    cardLayout?: DeckCardLayout;
    cards?: DeckCardInfo[];
    name?: string;
    sideboard?: DeckCardInfo[];
    sideboardLayout?: DeckCardLayout;
}

export interface DeckPutResult {
    stored?: boolean;
    updatedAt?: number;
}

export interface DeckSourceStatus {
    site?: string;
    status?: string;
}

export interface DeckSyncEntry {
    data?: string;
    deleted?: boolean;
    id?: string;
    name?: string;
    updatedAt?: number;
}

export interface DeckValidationResult {
    commanderBrackets?: BracketInfo[];
    deckType?: string;
    edhPowerCards?: string[];
    edhPowerDetails?: string[];
    edhPowerLevel?: number;
    errors?: ErrorInfo[];
    name?: string;
    partlyValid?: boolean;
    shortName?: string;
    valid?: boolean;
}

export interface DeckView extends Serializable {
    cards?: { [index: string]: SimpleCardView };
    name?: string;
    sideboard?: { [index: string]: SimpleCardView };
}

export interface DraftClientMessage extends Serializable {
    draftPickView?: DraftPickView;
    draftView?: DraftView;
}

export interface DraftPickView extends Serializable {
    booster?: { [index: string]: SimpleCardView };
    picking?: boolean;
    picks?: { [index: string]: SimpleCardView };
    timeout?: number;
}

export interface DraftView extends Serializable {
    boosterNum?: number;
    cardNum?: number;
    isCube?: boolean;
    players?: string[];
    setCodes?: string[];
    setNames?: string[];
}

export interface DungeonView extends CommandObjectView, Serializable {
    expansionSetCode?: string;
    id?: string;
    imageFileName?: string;
    imageNumber?: number;
    name?: string;
    playableStats?: PlayableObjectStats;
    rules?: string[];
}

export interface EmblemView extends CommandObjectView, Serializable {
    cardNumber?: string;
    expansionSetCode?: string;
    id?: string;
    imageFileName?: string;
    imageNumber?: number;
    name?: string;
    playableStats?: PlayableObjectStats;
    rules?: string[];
    usesVariousArt?: boolean;
}

export interface ErrorInfo {
    cardName?: string;
    group?: string;
    message?: string;
    type?: string;
}

export interface ExpansionSetInfo {
    blockName?: string;
    hasBasicLands?: boolean;
    hasBoosters?: boolean;
    name?: string;
    releaseDate?: number;
    setCode?: string;
    type?: string;
}

export interface FeedbackEntry {
    context?: string;
    createdAt?: number;
    handledAt?: number;
    handledBy?: string;
    id?: number;
    message?: string;
    note?: string;
    status?: string;
    title?: string;
    type?: string;
    userName?: string;
}

export interface GameClientMessage extends Serializable {
    cardsView1?: { [index: string]: CardView };
    cardsView2?: { [index: string]: CardView };
    choice?: Choice;
    flag?: boolean;
    gameView?: GameView;
    max?: number;
    message?: string;
    messages?: MultiAmountMessage[];
    min?: number;
    options?: { [index: string]: Serializable };
    targets?: string[];
}

export interface GameEndView extends Serializable {
    additionalInfo?: string;
    clientPlayer?: PlayerView;
    endTime?: DateAsNumber;
    gameInfo?: string;
    loses?: number;
    matchInfo?: string;
    matchView?: MatchView;
    players?: PlayerView[];
    startTime?: DateAsNumber;
    wins?: number;
    winsNeeded?: number;
    won?: boolean;
}

export interface GameEventView extends Serializable {
    amount?: number;
    combat?: boolean;
    counter?: string;
    fromZone?: string;
    kind?: GameEventKind;
    playerId?: string;
    seq?: number;
    sourceId?: string;
    sourceName?: string;
    targetId?: string;
    targetName?: string;
    toZone?: string;
}

export interface GameRecord {
    first?: string;
    n?: number;
    seed?: number;
    turns?: number;
    winner?: string;
}

export interface GameTypeView extends Serializable {
    maxPlayers?: number;
    minPlayers?: number;
    name?: string;
    numTeams?: number;
    playersPerTeam?: number;
    useAttackOption?: boolean;
    useRange?: boolean;
}

export interface GameView extends Serializable {
    activePlayerId?: string;
    activePlayerName?: string;
    attackOption?: MultiplayerAttackOption;
    bufferTime?: number;
    canPlayObjects?: PlayableObjectsList;
    combat?: CombatGroupView[];
    companion?: RevealedView[];
    events?: GameEventView[];
    exiles?: { [index: string]: CardView }[];
    gameCycle?: number;
    lookedAt?: LookedAtView[];
    myHand?: { [index: string]: CardView };
    myHelperEmblems?: { [index: string]: CardView };
    myPlayerId?: string;
    opponentHands?: { [index: string]: { [index: string]: SimpleCardView } };
    phase?: TurnPhase;
    players?: PlayerView[];
    priorityPlayerName?: string;
    priorityTime?: number;
    rangeOfInfluence?: RangeOfInfluence;
    revealed?: RevealedView[];
    rollbackTurnsAllowed?: boolean;
    special?: boolean;
    stack?: { [index: string]: CardView };
    step?: PhaseStep;
    totalEffectsCount?: number;
    totalErrorsCount?: number;
    turn?: number;
    watchedHands?: { [index: string]: { [index: string]: SimpleCardView } };
}

export interface GithubSignInResult {
    nameProblem?: string;
    rememberToken?: string;
    suggestedName?: string;
    userName?: string;
}

export interface ImportedDeck {
    author?: string;
    commanders?: DeckCardInfo[];
    companion?: DeckCardInfo[];
    cover?: DeckCardInfo;
    description?: string;
    format?: string;
    main?: DeckCardInfo[];
    maybeboardCount?: number;
    name?: string;
    remoteId?: string;
    remoteUpdatedAt?: number;
    side?: DeckCardInfo[];
    site?: string;
    url?: string;
}

export interface Invite {
    code?: string;
    createdAt?: number;
    createdBy?: string;
    expiresAt?: number;
    maxUses?: number;
    note?: string;
    revoked?: boolean;
    usedBy?: string;
    uses?: number;
}

export interface InviteCheck {
    expiresAt?: number;
    reason?: string;
    valid?: boolean;
}

export interface JobResult {
    draws?: number;
    errors?: number;
    games?: GameRecord[];
    turns?: number;
    winsA?: number;
    winsB?: number;
}

export interface JobSpec {
    deckA?: DeckCardLists;
    deckB?: DeckCardLists;
    deckType?: string;
    gameType?: string;
    games?: number;
    maxTurns?: number;
    nameA?: string;
    nameB?: string;
    seed?: number;
    skillA?: number;
    skillB?: number;
}

export interface LargeMessage {
    at?: number;
    chars?: number;
    method?: string;
    objectId?: string;
}

export interface Leaderboard {
    board?: string;
    entries?: LeaderboardEntry[];
    label?: string;
    mode?: string;
    scope?: string;
    total?: number;
    v1HigherIsBetter?: boolean;
    v1Label?: string;
    v2HigherIsBetter?: boolean;
    v2Label?: string;
    yours?: LeaderboardEntry;
}

export interface LeaderboardEntry {
    at?: number;
    attempts?: number;
    detail?: string;
    rank?: number;
    user?: string;
    v1?: number;
    v2?: number;
    you?: boolean;
}

export interface LookedAtView extends Serializable {
    cards?: { [index: string]: SimpleCardView };
    name?: string;
}

export interface MageInt extends Serializable, Copyable<MageInt> {
    baseValue?: number;
    boostedValue?: number;
    cardValue?: string;
    modifiedBaseValue?: number;
}

export interface MageVersion extends Serializable, Comparable<MageVersion> {
    buildTime?: string;
    major?: number;
    minor?: number;
    release?: number;
    releaseInfo?: string;
}

export interface MaintenancePlan {
    announcedAt?: number;
    by?: string;
    message?: string;
    restartWhenIdle?: boolean;
    startsAt?: number;
}

export interface ManaPoolView extends Serializable {
    black?: number;
    blue?: number;
    colorless?: number;
    green?: number;
    red?: number;
    white?: number;
}

export interface MatchView extends Serializable {
    deckType?: string;
    endTime?: DateAsNumber;
    gameType?: string;
    games?: string[];
    isTournament?: boolean;
    matchId?: string;
    matchName?: string;
    players?: string;
    rated?: boolean;
    replayAvailable?: boolean;
    result?: string;
    startTime?: DateAsNumber;
    tableId?: string;
}

export interface MultiAmountMessage extends Serializable, Copyable<MultiAmountMessage> {
    defaultValue?: number;
    max?: number;
    message?: string;
    min?: number;
}

export interface ObjectColor extends Serializable, Copyable<ObjectColor>, Comparable<ObjectColor> {
    black?: boolean;
    blue?: boolean;
    green?: boolean;
    red?: boolean;
    white?: boolean;
}

export interface PermanentView extends CardView {
    attachedControllerDiffers?: boolean;
    attachedTo?: string;
    attachedToPermanent?: boolean;
    attachments?: string[];
    cloaked?: boolean;
    controlled?: boolean;
    copy?: boolean;
    damage?: number;
    disguised?: boolean;
    flipped?: boolean;
    manifested?: boolean;
    morphed?: boolean;
    mutateView?: { [index: string]: CardView };
    mutated?: boolean;
    nameController?: string;
    nameOwner?: string;
    original?: CardView;
    phasedIn?: boolean;
    summoningSickness?: boolean;
    tapped?: boolean;
}

export interface PlaneView extends CommandObjectView, Serializable {
    expansionSetCode?: string;
    id?: string;
    imageFileName?: string;
    imageNumber?: number;
    name?: string;
    playableStats?: PlayableObjectStats;
    rules?: string[];
}

export interface PlayableObjectRecord extends Serializable, Copyable<PlayableObjectRecord> {
    id?: string;
    value?: string;
}

export interface PlayableObjectStats extends Serializable, Copyable<PlayableObjectStats> {
    basicCastAbilities?: PlayableObjectRecord[];
    basicManaAbilities?: PlayableObjectRecord[];
    basicPlayAbilities?: PlayableObjectRecord[];
    other?: PlayableObjectRecord[];
}

export interface PlayableObjectsList extends Serializable, Copyable<PlayableObjectsList> {
    objects?: { [index: string]: PlayableObjectStats };
}

export interface PlayerReport {
    closedAt?: number;
    createdAt?: number;
    details?: string;
    gameId?: string;
    id?: number;
    open?: boolean;
    reason?: string;
    reported?: string;
    reporter?: string;
    resolution?: string;
}

export interface PlayerView extends Serializable {
    attachments?: string[];
    battlefield?: { [index: string]: PermanentView };
    bufferTimeLeft?: number;
    commandList?: CommandObjectView[];
    controlled?: boolean;
    counters?: CounterView[];
    designationNames?: string[];
    exile?: { [index: string]: CardView };
    graveyard?: { [index: string]: CardView };
    handCount?: number;
    hasLeft?: boolean;
    hasPriority?: boolean;
    helperCards?: { [index: string]: CardView };
    initiative?: boolean;
    isActive?: boolean;
    isHuman?: boolean;
    libraryCount?: number;
    life?: number;
    manaPool?: ManaPoolView;
    monarch?: boolean;
    name?: string;
    passedAllTurns?: boolean;
    passedTurn?: boolean;
    passedUntilEndOfTurn?: boolean;
    passedUntilEndStepBeforeMyTurn?: boolean;
    passedUntilNextMain?: boolean;
    passedUntilStackResolved?: boolean;
    playerId?: string;
    priorityTimeLeftSecs?: number;
    priorityTimeSavedTimeMs?: number;
    sideboard?: { [index: string]: CardView };
    statesSavedSize?: number;
    timerActive?: boolean;
    topCard?: CardView;
    userData?: UserData;
    wins?: number;
    winsNeeded?: number;
}

export interface ReplayInfo {
    deckType?: string;
    endedAt?: number;
    events?: number;
    gameId?: string;
    gameType?: string;
    players?: ReplayPlayer[];
    result?: string;
    startedAt?: number;
    tableId?: string;
    tableName?: string;
    truncated?: boolean;
    turns?: number;
    version?: number;
}

export interface ReplayPlayer {
    human?: boolean;
    name?: string;
}

export interface RevealedView extends Serializable {
    cards?: { [index: string]: CardView };
    name?: string;
}

export interface RoomUsersView extends Serializable {
    numberActiveGames?: number;
    numberGameThreads?: number;
    numberMaxGames?: number;
    usersView?: UsersView[];
}

export interface RoundView extends Serializable {
    games?: TournamentGameView[];
}

export interface SeatView extends Serializable {
    constructedRating?: number;
    flagName?: string;
    generalRating?: number;
    history?: string;
    limitedRating?: number;
    playerId?: string;
    playerName?: string;
    playerType?: PlayerType;
}

export interface SelectableObjectView {
}

export interface Serializable {
}

export interface ServerInfo {
    access?: string;
    accounts?: boolean;
    career?: boolean;
    deckSync?: boolean;
    gamesRunning?: number;
    gamesWaiting?: number;
    github?: boolean;
    mail?: boolean;
    maintenanceAt?: number;
    maintenanceMessage?: string;
    maxGames?: number;
    maxPasswordLength?: number;
    maxUserNameLength?: number;
    minPasswordLength?: number;
    minUserNameLength?: number;
    notice?: string;
    registration?: string;
    serverName?: string;
    testMode?: boolean;
}

export interface ServerSetting {
    changed?: boolean;
    changedAt?: number;
    changedBy?: string;
    configValue?: string;
    locked?: string;
    name?: string;
    value?: string;
}

export interface ServerSettings {
    discordLobby?: boolean;
    discordStaff?: boolean;
    maintenance?: MaintenancePlan;
    settings?: ServerSetting[];
}

export interface ServerState extends Serializable {
    cardsContentVersion?: number;
    deckTypes?: string[];
    draftCubes?: string[];
    expansionsContentVersion?: number;
    gameTypes?: GameTypeView[];
    playerTypes?: PlayerType[];
    testMode?: boolean;
    tournamentTypes?: TournamentTypeView[];
    version?: MageVersion;
}

export interface ServerStats {
    activeGames?: number;
    backup?: BackupRun;
    bridge?: BridgeSnapshot;
    commit?: string;
    heapMax?: number;
    heapUsed?: number;
    maintenance?: MaintenancePlan;
    now?: number;
    processCpu?: number;
    processors?: number;
    systemLoad?: number;
    tables?: number;
    threads?: number;
    uptimeMillis?: number;
    usersOnline?: number;
    version?: string;
}

export interface SimpleCardView extends Serializable, SelectableObjectView {
    cardNumber?: string;
    expansionSetCode?: string;
    gameObject?: boolean;
    id?: string;
    isChoosable?: boolean;
    isSelected?: boolean;
    playableStats?: PlayableObjectStats;
    usesVariousArt?: boolean;
}

export interface SkipPrioritySteps extends Serializable {
    beforeCombat?: boolean;
    draw?: boolean;
    endOfCombat?: boolean;
    endOfTurn?: boolean;
    main1?: boolean;
    main2?: boolean;
    upkeep?: boolean;
}

export interface SoloSave {
    career?: boolean;
    created?: number;
    gameType?: string;
    id?: string;
    kind?: string;
    opponents?: string;
    running?: boolean;
    title?: string;
    turn?: number;
    updated?: number;
}

export interface SoloTable {
    tableId?: string;
}

export interface StackAbilityView extends CardView {
    sourceCard?: CardView;
}

export interface StaffList {
    accounts?: boolean;
    members?: StaffMember[];
    owner?: string;
}

export interface StaffMe {
    actor?: string;
    role?: string;
}

export interface StaffMember {
    grantedAt?: number;
    grantedBy?: string;
    key?: string;
    name?: string;
    role?: string;
}

export interface StaffResetCode {
    code?: string;
    email?: string;
    emailed?: boolean;
    expiresAt?: number;
}

export interface TableClientMessage extends Serializable {
    currentTableId?: string;
    deck?: DeckView;
    flag?: boolean;
    gameId?: string;
    parentTableId?: string;
    playerId?: string;
    roomId?: string;
    time?: number;
}

export interface TableView extends Serializable {
    additionalInfoFull?: string;
    additionalInfoShort?: string;
    controllerName?: string;
    createTime?: DateAsNumber;
    deckType?: string;
    gameType?: string;
    games?: string[];
    isTournament?: boolean;
    limited?: boolean;
    minimumRating?: string;
    passworded?: boolean;
    quitRatio?: string;
    rated?: boolean;
    seats?: SeatView[];
    seatsInfo?: string;
    skillLevel?: SkillLevel;
    spectatorsAllowed?: boolean;
    tableId?: string;
    tableName?: string;
    tableState?: TableState;
    tableStateText?: string;
}

export interface TimedTwist extends Serializable {
    card?: string;
    gainLife?: number;
    life?: number;
    message?: string;
    then?: CareerTwists;
    turn?: number;
    when?: string;
    who?: string;
}

export interface TournamentGameView extends Serializable {
    gameId?: string;
    matchId?: string;
    players?: string;
    result?: string;
    roundNum?: number;
    state?: string;
    tableId?: string;
}

export interface TournamentPlayerView extends Serializable, Comparable<any> {
    flagName?: string;
    history?: string;
    name?: string;
    points?: number;
    quit?: boolean;
    results?: string;
    state?: string;
}

export interface TournamentTypeView extends Serializable {
    cubeBooster?: boolean;
    draft?: boolean;
    elimination?: boolean;
    jumpstart?: boolean;
    limited?: boolean;
    maxPlayers?: number;
    minPlayers?: number;
    name?: string;
    numBoosters?: number;
    random?: boolean;
    reshuffled?: boolean;
    richMan?: boolean;
}

export interface TournamentView extends Serializable {
    constructionTime?: number;
    endTime?: DateAsNumber;
    players?: TournamentPlayerView[];
    rounds?: RoundView[];
    runningInfo?: string;
    serverTime?: DateAsNumber;
    startTime?: DateAsNumber;
    stepStartTime?: DateAsNumber;
    tournamentName?: string;
    tournamentState?: string;
    tournamentType?: string;
    watchingAllowed?: boolean;
}

export interface UserData extends Serializable {
    allowRequestShowHandCards?: boolean;
    askMoveToGraveOrder?: boolean;
    autoOrderTrigger?: boolean;
    autoTargetLevel?: number;
    avatarId?: number;
    confirmEmptyManaPool?: boolean;
    constructedRating?: number;
    flagName?: string;
    generalRating?: number;
    groupId?: number;
    limitedRating?: number;
    manaPoolAutomatic?: boolean;
    manaPoolAutomaticRestricted?: boolean;
    matchHistory?: string;
    matchQuitRatio?: number;
    passPriorityActivation?: boolean;
    passPriorityCast?: boolean;
    requestedHandPlayersList?: { [index: string]: string[] };
    tourneyHistory?: string;
    tourneyQuitRatio?: number;
    useFirstManaAbility?: boolean;
    useSameSettingsForReplacementEffects?: boolean;
    userIdStr?: string;
    userSkipPrioritySteps?: UserSkipPrioritySteps;
}

export interface UserRequestMessage extends Serializable {
    button1Action?: PlayerAction;
    button1Text?: string;
    button2Action?: PlayerAction;
    button2Text?: string;
    button3Action?: PlayerAction;
    button3Text?: string;
    gameId?: string;
    matchId?: string;
    message?: string;
    relatedUserId?: string;
    relatedUserName?: string;
    roomId?: string;
    tableId?: string;
    title?: string;
    tournamentId?: string;
    windowSizeRatio?: number;
}

export interface UserSkipPrioritySteps extends Serializable {
    opponentTurn?: SkipPrioritySteps;
    stopOnAllEndPhases?: boolean;
    stopOnAllMainPhases?: boolean;
    stopOnDeclareAttackers?: boolean;
    stopOnDeclareBlockersWithAnyPermanents?: boolean;
    stopOnDeclareBlockersWithZeroPermanents?: boolean;
    stopOnStackNewObjects?: boolean;
    yourTurn?: SkipPrioritySteps;
}

export interface UserView extends Serializable {
    clientVersion?: string;
    email?: string;
    gameInfo?: string;
    host?: string;
    lastActivity?: DateAsNumber;
    muteChatUntil?: DateAsNumber;
    sessionId?: string;
    timeConnected?: DateAsNumber;
    userIdStr?: string;
    userName?: string;
    userState?: string;
}

export interface UsersView extends Serializable {
    avatarId?: number;
    constructedRating?: number;
    flagName?: string;
    generalRating?: number;
    infoGames?: string;
    infoPing?: string;
    limitedRating?: number;
    matchHistory?: string;
    matchQuitRatio?: number;
    tourneyHistory?: string;
    tourneyQuitRatio?: number;
    userName?: string;
}

export interface Wildcards {
    common?: number;
    mythic?: number;
    rare?: number;
    uncommon?: number;
}

export type AbilityType = "PLAY_LAND" | "SPELL" | "STATIC" | "EVASION" | "ACTIVATED_NONMANA" | "ACTIVATED_MANA" | "TRIGGERED_NONMANA" | "TRIGGERED_MANA" | "SPECIAL_ACTION" | "SPECIAL_MANA_PAYMENT";

export type AdviceKind = "PLAY" | "PASS" | "ATTACK" | "NO_ATTACK" | "BLOCK" | "NO_BLOCK" | "NONE";

export type ArtRect = "NORMAL" | "RETRO" | "AFTERMATH_TOP" | "AFTERMATH_BOTTOM" | "SPLIT_LEFT" | "SPLIT_RIGHT" | "SPLIT_FUSED" | "FULL_LENGTH_LEFT" | "FULL_LENGTH_RIGHT";

export type CardType = "ARTIFACT" | "BATTLE" | "CONSPIRACY" | "CREATURE" | "DUNGEON" | "ENCHANTMENT" | "INSTANT" | "LAND" | "PHENOMENON" | "PLANE" | "PLANESWALKER" | "SCHEME" | "SORCERY" | "KINDRED" | "VANGUARD";

export type ChoiceHintType = "TEXT" | "CARD" | "CARD_DUNGEON" | "GAME_OBJECT";

export type DateAsNumber = number;

export type FrameStyle = "M15_NORMAL" | "BFZ_FULL_ART_BASIC" | "KLD_INVENTION" | "ZEN_FULL_ART_BASIC" | "MPRP_FULL_ART_BASIC" | "MPOP_FULL_ART_BASIC" | "UNH_FULL_ART_BASIC" | "UGL_FULL_ART_BASIC" | "UST_FULL_ART_BASIC" | "ANA_FULL_ART_BASIC" | "LEA_ORIGINAL_DUAL_LAND_ART_BASIC" | "RETRO";

export type GameEventKind = "DAMAGE" | "LIFE_GAIN" | "LIFE_LOSS" | "ZONE" | "COUNTERS_ADDED" | "COUNTERS_REMOVED" | "TAP" | "UNTAP" | "CAST" | "ATTACK" | "BLOCK" | "DRAW" | "TOKEN";

export type MageObjectType = "ABILITY_STACK_FROM_CARD" | "ABILITY_STACK_FROM_TOKEN" | "CARD" | "COPY_CARD" | "TOKEN" | "SPELL" | "PERMANENT" | "DUNGEON" | "EMBLEM" | "COMMANDER" | "DESIGNATION" | "PLANE" | "NULL";

export type ManaType = "BLACK" | "BLUE" | "GREEN" | "RED" | "WHITE" | "GENERIC" | "COLORLESS";

export type MatchBufferTime = "NONE" | "SEC__01" | "SEC__02" | "SEC__03" | "SEC__05" | "SEC__10" | "SEC__15" | "SEC__20" | "SEC__25" | "SEC__30";

export type MatchTimeLimit = "NONE" | "MIN___5" | "MIN__10" | "MIN__15" | "MIN__20" | "MIN__25" | "MIN__30" | "MIN__35" | "MIN__40" | "MIN__45" | "MIN__50" | "MIN__55" | "MIN__60" | "MIN__90" | "MIN_120";

export type MessageColor = "BLACK" | "RED" | "GREEN" | "BLUE" | "ORANGE" | "YELLOW";

export type MessageType = "USER_INFO" | "STATUS" | "GAME" | "TALK" | "WHISPER_FROM" | "WHISPER_TO";

export type MulliganType = "GAME_DEFAULT" | "VANCOUVER" | "PARIS" | "LONDON" | "SMOOTHED_LONDON" | "CANADIAN_HIGHLANDER";

export type MultiplayerAttackOption = "MULTIPLE" | "LEFT" | "RIGHT";

export type PhaseStep = "UNTAP" | "UPKEEP" | "DRAW" | "PRECOMBAT_MAIN" | "BEGIN_COMBAT" | "DECLARE_ATTACKERS" | "DECLARE_BLOCKERS" | "FIRST_COMBAT_DAMAGE" | "COMBAT_DAMAGE" | "END_COMBAT" | "POSTCOMBAT_MAIN" | "END_TURN" | "CLEANUP";

export type PlayerAction = "PASS_PRIORITY_UNTIL_MY_NEXT_TURN" | "PASS_PRIORITY_UNTIL_TURN_END_STEP" | "PASS_PRIORITY_UNTIL_NEXT_MAIN_PHASE" | "PASS_PRIORITY_UNTIL_NEXT_TURN" | "PASS_PRIORITY_UNTIL_NEXT_TURN_SKIP_STACK" | "PASS_PRIORITY_UNTIL_STACK_RESOLVED" | "PASS_PRIORITY_UNTIL_END_STEP_BEFORE_MY_NEXT_TURN" | "PASS_PRIORITY_CANCEL_ALL_ACTIONS" | "TRIGGER_AUTO_ORDER_ABILITY_FIRST" | "TRIGGER_AUTO_ORDER_NAME_FIRST" | "TRIGGER_AUTO_ORDER_ABILITY_LAST" | "TRIGGER_AUTO_ORDER_NAME_LAST" | "TRIGGER_AUTO_ORDER_RESET_ALL" | "ROLLBACK_TURNS" | "UNDO" | "CONCEDE" | "MANA_AUTO_PAYMENT_ON" | "MANA_AUTO_PAYMENT_OFF" | "MANA_AUTO_PAYMENT_RESTRICTED_ON" | "MANA_AUTO_PAYMENT_RESTRICTED_OFF" | "USE_FIRST_MANA_ABILITY_ON" | "USE_FIRST_MANA_ABILITY_OFF" | "RESET_AUTO_SELECT_REPLACEMENT_EFFECTS" | "REVOKE_PERMISSIONS_TO_SEE_HAND_CARDS" | "REQUEST_PERMISSION_TO_SEE_HAND_CARDS" | "REQUEST_PERMISSION_TO_ROLLBACK_TURN" | "ADD_PERMISSION_TO_SEE_HAND_CARDS" | "ADD_PERMISSION_TO_ROLLBACK_TURN" | "DENY_PERMISSION_TO_ROLLBACK_TURN" | "PERMISSION_REQUESTS_ALLOWED_ON" | "PERMISSION_REQUESTS_ALLOWED_OFF" | "REQUEST_AUTO_ANSWER_ID_YES" | "REQUEST_AUTO_ANSWER_ID_NO" | "REQUEST_AUTO_ANSWER_TEXT_YES" | "REQUEST_AUTO_ANSWER_TEXT_NO" | "REQUEST_AUTO_ANSWER_RESET_ALL" | "CLIENT_DOWNLOAD_SYMBOLS" | "CLIENT_QUIT_TOURNAMENT" | "CLIENT_QUIT_DRAFT_TOURNAMENT" | "CLIENT_CONCEDE_GAME" | "CLIENT_CONCEDE_MATCH" | "CLIENT_STOP_WATCHING" | "CLIENT_DISCONNECT_FULL" | "CLIENT_DISCONNECT_KEEP_GAMES" | "CLIENT_EXIT_FULL" | "CLIENT_EXIT_KEEP_GAMES" | "CLIENT_REMOVE_TABLE" | "CLIENT_DOWNLOAD_CARD_IMAGES" | "CLIENT_RECONNECT" | "CLIENT_REPLAY_ACTION" | "HOLD_PRIORITY" | "UNHOLD_PRIORITY" | "VIEW_LIMITED_DECK" | "VIEW_SIDEBOARD" | "TOGGLE_RECORD_MACRO";

export type PlayerType = "HUMAN" | "COMPUTER_DRAFT_BOT" | "COMPUTER_MONTE_CARLO" | "COMPUTER_MAD";

export type RangeOfInfluence = "ONE" | "TWO" | "ALL";

export type Rarity = "LAND" | "COMMON" | "UNCOMMON" | "RARE" | "MYTHIC" | "SPECIAL" | "BONUS";

export type SkillLevel = "BEGINNER" | "CASUAL" | "SERIOUS";

export type SoundToPlay = "PlayerLeft" | "PlayerQuitTournament" | "PlayerSubmittedDeck" | "PlayerWhispered";

export type SubType = "ADVENTURE" | "ARCANE" | "LESSON" | "OMEN" | "TRAP" | "SIEGE" | "FOREST" | "ISLAND" | "MOUNTAIN" | "PLAINS" | "SWAMP" | "CAVE" | "DESERT" | "GATE" | "LAIR" | "LOCUS" | "MINE" | "PLANET" | "POWER_PLANT" | "SPHERE" | "TOWER" | "TOWN" | "URZAS" | "AURA" | "BACKGROUND" | "CARTOUCHE" | "CASE" | "CLASS" | "CURSE" | "PLAN" | "ROLE" | "ROOM" | "RUNE" | "SAGA" | "SHARD" | "SHRINE" | "ATTRACTION" | "BLOOD" | "BOBBLEHEAD" | "BOOK" | "CLUE" | "COMMUNICATOR" | "CONTRAPTION" | "EQUIPMENT" | "FOOD" | "FORTIFICATION" | "GOLD" | "HEARTWOOD" | "INCUBATOR" | "INFINITY" | "JUNK" | "LANDER" | "MAP" | "MUTAGEN" | "POWERSTONE" | "SPACECRAFT" | "STONE" | "TREASURE" | "VEHICLE" | "VIBRANIUM" | "ADVISOR" | "AETHERBORN" | "ALIEN" | "ALLY" | "ANDORIAN" | "ANGEL" | "ANTELOPE" | "ANZELLAN" | "APE" | "AQUALISH" | "ARCHER" | "ARCHON" | "ARCONA" | "ARMADILLO" | "ARMY" | "ARTIFICER" | "ASSASSIN" | "ASSEMBLY_WORKER" | "ASTARTES" | "ATAT" | "ATHLETE" | "ATOG" | "AUROCHS" | "AUTOBOT" | "AVATAR" | "AZRA" | "BADGER" | "BALLOON" | "BARABEL" | "BARBARIAN" | "BARD" | "BASILISK" | "BAT" | "BEAR" | "BEAST" | "BEAVER" | "BEEBLE" | "BEHOLDER" | "BERSERKER" | "BIRD" | "BISON" | "BITH" | "BLINKMOTH" | "BOAR" | "BORG" | "BRAINIAC" | "BRINGER" | "BRUSHWAGG" | "CTAN" | "CAITIAN" | "CALAMARI" | "CAMARID" | "CAMEL" | "CAPYBARA" | "CARIBOU" | "CARRIER" | "CAT" | "CENTAUR" | "CEREAN" | "CHILD" | "CHIMERA" | "CHISS" | "CITIZEN" | "CLAMFOLK" | "CLERIC" | "CLOWN" | "COCKATRICE" | "CONSTRUCT" | "COW" | "COWARD" | "COYOTE" | "CRAB" | "CROCODILE" | "CROLUTE" | "CUSTODES" | "CYBERMAN" | "CYBORG" | "CYCLOPS" | "DALEK" | "DATHOMIRIAN" | "DAUTHI" | "DEMIGOD" | "DEMON" | "DESERTER" | "DETECTIVE" | "DEVIL" | "DINOSAUR" | "DJINN" | "DOCTOR" | "DOG" | "DRAGON" | "DRAKE" | "DREADNOUGHT" | "DRIX" | "DROID" | "DRONE" | "DRUID" | "DRYAD" | "DWARF" | "ECHIDNA" | "EFREET" | "EGG" | "ELDER" | "ELDRAZI" | "ELEMENTAL" | "ELEPHANT" | "ELF" | "ELK" | "EMPLOYEE" | "ETERNAL" | "EWOK" | "EXPANSION_SYMBOL" | "EYE" | "FAERIE" | "FERRET" | "FISH" | "FLAGBEARER" | "FOX" | "FRACTAL" | "FROG" | "FUNGUS" | "GAMER" | "GAMMA" | "GAMORREAN" | "GAND" | "GARGOYLE" | "GERM" | "GIANT" | "GIRAFFE" | "GITH" | "GLIMMER" | "GNOLL" | "GNOME" | "GOAT" | "GOBLIN" | "GOD" | "GOLEM" | "GORGON" | "GORN" | "GRAVEBORN" | "GREMLIN" | "GRIFFIN" | "GUEST" | "GUNGAN" | "HAG" | "HALFLING" | "HAMSTER" | "HARPY" | "HEDGEHOG" | "HELLION" | "HERO" | "HIPPO" | "HIPPOGRIFF" | "HOMARID" | "HOMUNCULUS" | "HORROR" | "HORSE" | "HUMAN" | "HUNTER" | "HUTT" | "HYDRA" | "HYENA" | "ILLUSION" | "IMP" | "INCARNATION" | "INHUMAN" | "INKLING" | "INQUISITOR" | "INSECT" | "ITHORIAN" | "JACKAL" | "JAWA" | "JEDI" | "JELLYFISH" | "JUGGERNAUT" | "KALEESH" | "KANGAROO" | "KAVU" | "KELDOR" | "KELPIEN" | "KILLBOT" | "KIRIN" | "KITHKIN" | "KLINGON" | "KNIGHT" | "KOBOLD" | "KOORIVAR" | "KOR" | "KRAKEN" | "KREE" | "LADYOFPROPERETIQUETTE" | "LAMIA" | "LAMMASU" | "LANTHANITE" | "LEECH" | "LEMUR" | "LEVIATHAN" | "LHURGOYF" | "LICID" | "LIZARD" | "LLAMA" | "LOBSTER" | "MANTELLIAN" | "MANTICORE" | "MASTICORE" | "MERCENARY" | "MERFOLK" | "METATHRAN" | "MINION" | "MINOTAUR" | "MIRIALAN" | "MITE" | "MOLE" | "MONGER" | "MONGOOSE" | "MONK" | "MONKEY" | "MOOGLE" | "MOONFOLK" | "MOUNT" | "MOUSE" | "MUTANT" | "MYR" | "MYSTIC" | "NAUTILUS" | "NAUTOLAN" | "NECRON" | "NEIMOIDIAN" | "NEPHILIM" | "NIGHTMARE" | "NIGHTSTALKER" | "NINJA" | "NOBLE" | "NOGGLE" | "NOMAD" | "NYMPH" | "OCTOPUS" | "OFFICER" | "OGRE" | "OOZE" | "ORB" | "ORC" | "ORGG" | "ORION" | "ORTOLAN" | "OTTER" | "OUPHE" | "OX" | "OYSTER" | "PANGOLIN" | "PEASANT" | "PEGASUS" | "PENTAVITE" | "PERFORMER" | "PEST" | "PHELDDAGRIF" | "PHOENIX" | "PHYREXIAN" | "PILOT" | "PINCHER" | "PIRATE" | "PLANT" | "PLATYPUS" | "PORCUPINE" | "POSSUM" | "PRAETOR" | "PRIMARCH" | "PRISM" | "PROCESSOR" | "PUREBLOOD" | "Q" | "QU" | "QUARREN" | "RABBIT" | "RACCOON" | "RAIDER" | "RANGER" | "RAT" | "REBEL" | "REFLECTION" | "RHINO" | "RIGGER" | "ROBOT" | "RODIAN" | "ROGUE" | "ROMULAN" | "SABLE" | "SALAMANDER" | "SAMURAI" | "SAND" | "SAPROLING" | "SATYR" | "SCARECROW" | "SCIENTIST" | "SCION" | "SCORPION" | "SCOUT" | "SCULPTURE" | "SEAL" | "SERF" | "SERPENT" | "SERVO" | "SHADE" | "SHAMAN" | "SHAPESHIFTER" | "SHARK" | "SHEEP" | "SHIAR" | "SIREN" | "SITH" | "SKELETON" | "SKRULL" | "SKUNK" | "SLITH" | "SLIVER" | "SLOTH" | "SLUG" | "SNAIL" | "SNAKE" | "SOLDIER" | "SOLTARI" | "SORCERER" | "SPAWN" | "SPECTER" | "SPELLSHAPER" | "SPHINX" | "SPIDER" | "SPIKE" | "SPIRIT" | "SPLINTER" | "SPLITTER" | "SPONGE" | "SPY" | "SQUID" | "SQUIRREL" | "STARFISH" | "STARSHIP" | "SULLUSTAN" | "SURRAKAR" | "SURVIVOR" | "SYMBIOTE" | "SYNTH" | "TALOSIAN" | "TELLARITE" | "TENTACLE" | "TETRAVITE" | "THALAKOS" | "THOLIAN" | "THOPTER" | "THRULL" | "TIEFLING" | "TIME_LORD" | "TOSK" | "TOY" | "TRANDOSHAN" | "TREEFOLK" | "TRILOBITE" | "TRISKELAVITE" | "TROLL" | "TROOPER" | "TURTLE" | "TUSKEN" | "TWILEK" | "TYRANID" | "UGNAUGHT" | "UNICORN" | "UTROM" | "VAMPIRE" | "VARMINT" | "VEDALKEN" | "VILLAIN" | "VOLVER" | "VORTA" | "VULCAN" | "WALL" | "WALRUS" | "WARLOCK" | "WARRIOR" | "WEASEL" | "WEEQUAY" | "WEIRD" | "WEREWOLF" | "WHALE" | "WIZARD" | "WOLF" | "WOLVERINE" | "WOMBAT" | "WOOKIEE" | "WORM" | "WRAITH" | "WURM" | "XINDI" | "YETI" | "ZABRAK" | "ZOMBIE" | "ZUBERA" | "AJANI" | "AMINATOU" | "ANGRATH" | "ARLINN" | "ARZAKON" | "ASHIOK" | "AURRA" | "BAHAMUT" | "BASRI" | "BOLAS" | "CALIX" | "CHANDRA" | "COMET" | "DACK" | "DAKKON" | "DARETTI" | "DAVRIEL" | "DELLIAN" | "DIHADA" | "DOMRI" | "DOOKU" | "DOVIN" | "DYFED" | "ELLYWICK" | "ELMINSTER" | "ELSPETH" | "ESTRID" | "FEROZ" | "FREYALISE" | "GARRUK" | "GIDEON" | "GREENSLEEVES" | "GRIST" | "GUFF" | "HUATLI" | "INZERVA" | "JACE" | "JARED" | "JAYA" | "JESKA" | "KAITO" | "KARN" | "KASMINA" | "KAYA" | "KIORA" | "KOTH" | "LILIANA" | "LOLTH" | "LUKE" | "LUKKA" | "MINSC" | "MORDENKAINEN" | "NAHIRI" | "NARSET" | "NIKO" | "NISSA" | "NIXILIS" | "OBI_WAN" | "OKO" | "QUINTORIUS" | "RAL" | "REY" | "ROWAN" | "SAHEELI" | "SAMUT" | "SARKHAN" | "SERRA" | "SIDIOUS" | "SIVITRI" | "SNOKE" | "SORIN" | "SZAT" | "TAMIYO" | "TASHA" | "TEFERI" | "TEYO" | "TEZZERET" | "TIBALT" | "TYVAR" | "UGIN" | "URZA" | "VENSER" | "VIVIEN" | "VRASKA" | "VRONOS" | "WILL" | "WINDGRACE" | "WORZEL" | "WRENN" | "XENAGOS" | "YANGGU" | "YANLING" | "YODA" | "ZARIEL";

export type SuperType = "BASIC" | "ELITE" | "LEGENDARY" | "ONGOING" | "SNOW" | "WORLD";

export type TableState = "WAITING" | "READY_TO_START" | "STARTING" | "DRAFTING" | "CONSTRUCTING" | "DUELING" | "SIDEBOARDING" | "FINISHED";

export type TimingOption = "BEGINNER" | "REGULAR" | "PROFESSIONAL" | "NONE";

export type TurnPhase = "BEGINNING" | "PRECOMBAT_MAIN" | "COMBAT" | "POSTCOMBAT_MAIN" | "END";

export type Zone = "HAND" | "GRAVEYARD" | "LIBRARY" | "BATTLEFIELD" | "STACK" | "EXILED" | "ALL" | "OUTSIDE" | "COMMAND";
