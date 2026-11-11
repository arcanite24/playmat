#!/usr/bin/env python3
"""Cards for the Rust engine (docs/rust/Engine.md): reads card classes in Mage.Sets and writes the ones the engine
can express as JSON, one object per card, named after the Java framework classes they use.

    python3 rust/tools/transpile.py [--pool FILE] [--out FILE] [--report FILE]

--pool names the cards to translate, one per line (default: every card in the Pioneer and Standard Career school
decks). A card statement the translator doesn't know makes the card unsupported; --report lists why, per card.
"""
import argparse
import glob
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
SETS = os.path.join(ROOT, 'Mage.Sets', 'src', 'mage', 'cards')
SUBTYPES = os.path.join(ROOT, 'Mage', 'src', 'main', 'java', 'mage', 'constants', 'SubType.java')
DEFAULT_OUT = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'cards', 'cards.json')
DEFAULT_FACTS = os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'cards', 'facts.json')

BASIC_LANDS = {
    'Plains': 'WhiteManaAbility',
    'Island': 'BlueManaAbility',
    'Swamp': 'BlackManaAbility',
    'Mountain': 'RedManaAbility',
    'Forest': 'GreenManaAbility',
}

# abilities used as X.getInstance() or new X() with no arguments
KEYWORDS = {
    'FlyingAbility', 'ReachAbility', 'HasteAbility', 'VigilanceAbility', 'TrampleAbility', 'FirstStrikeAbility',
    'CantBeCounteredSourceAbility', 'DoubleStrikeAbility', 'LifelinkAbility', 'DeathtouchAbility', 'DefenderAbility', 'MenaceAbility', 'FlashAbility', 'ReboundAbility', 'ImproviseAbility', 'DevoidAbility',
    'HexproofAbility', 'IndestructibleAbility', 'WhiteManaAbility', 'BlueManaAbility', 'BlackManaAbility',
    'RedManaAbility', 'GreenManaAbility', 'ColorlessManaAbility', 'CanBlockOnlyFlyingAbility', 'CantBeBlockedSourceAbility', 'CantBlockAbility', 'DauntAbility',
    'IslandwalkAbility',
    'UmbraArmorAbility',
    'ConvokeAbility', 'DayboundAbility', 'NightboundAbility',
}

# effects: class -> its constructor arguments, each (field name, kind); a kind of None means the argument is ignored
# (a text override: rule texts come from the card facts)
EFFECTS = {
    'DamageTargetEffect': [('amount', 'int'), (None, 'string')],
    'DrawCardSourceControllerEffect': [('amount', 'int'), (None, 'bool')],
    'GainLifeEffect': [('amount', 'int')],
    'DestroyTargetEffect': [],
    'CounterTargetEffect': [],
    'BoostTargetEffect': [('power', 'int'), ('toughness', 'int'), ('duration', 'duration')],
    'BoostSourceEffect': [('power', 'int'), ('toughness', 'int'), ('duration', 'duration'), (None, 'string')],
    'BoostControlledEffect': [('power', 'int'), ('toughness', 'int'), ('duration', 'duration'), ('filter', 'filter'),
                              ('exclude_source', 'bool')],
    'AddCountersSourceEffect': [('counter', 'counter')],
    'AddCountersTargetEffect': [('counter', 'counter')],
    'CreateTokenEffect': [('token', 'token'), ('amount', 'int'), ('tapped', 'bool'), ('attacking', 'bool')],
    'DamageWithPowerFromOneToAnotherTargetEffect': [],
    'GainAbilityAllEffect': [('ability', 'ability'), ('duration', 'duration'), ('filter', 'filter'),
                             ('exclude_source', 'bool')],
    'ConditionalOneShotEffect': [('effect', 'effect'), ('otherwise', 'effect'), ('condition', 'condition'),
                                 (None, 'string')],
    'DamagePlayersEffect': [('amount', 'int'), ('controller', 'target_controller'), (None, 'string')],
    'SacrificeSourceEffect': [],
    'DoIfCostPaid': [('effect', 'effect'), ('cost', 'cost')],
    'GainAbilityControllerEffect': [('ability', 'ability')],
    'GainAbilityControlledEffect': [('ability', 'ability'), ('duration', 'duration'), ('filter', 'filter'),
                                    ('exclude_source', 'bool')],
    'GainAbilityTargetEffect': [('ability', 'ability'), ('duration', 'duration')],
    'GainAbilitySourceEffect': [('ability', 'ability'), ('duration', 'duration')],
    'LoseAbilityAllEffect': [('ability', 'ability'), ('duration', 'duration'), ('filter', 'filter')],
    'ConditionalContinuousEffect': [('effect', 'effect'), ('condition', 'condition'), (None, 'string')],
    'SetBasePowerToughnessSourceEffect': [('count', 'count')],
    'ExileTargetIfDiesEffect': [],
    'TapSourceUnlessPaysEffect': [('life', 'life_cost')],
    'LoseLifeOpponentsEffect': [('amount', 'int')],
    'LoseLifeSourceControllerEffect': [('amount', 'int')],
    'LoseLifeTargetEffect': [('amount', 'int')],
    'DestroyAllEffect': [('filter', 'filter')],
    'DamageAllEffect': [('amount', 'int'), ('filter', 'filter')],
    'ReturnFromYourGraveyardToBattlefieldAllEffect': [('filter', 'filter')],
    'BoostAllEffect': [('power', 'int'), ('toughness', 'int'), ('duration', 'duration'), ('filter', 'filter'),
                       ('exclude_source', 'bool')],
    'AddCountersAllEffect': [('counter', 'counter'), ('filter', 'filter')],
    'ExileTargetEffect': [],
    'ExileAndGainLifeEqualPowerTargetEffect': [],
    'ReturnToHandTargetEffect': [],
    'ReturnToHandSourceEffect': [('battlefield_only', 'bool')],
    'LookLibraryTopCardTargetPlayerEffect': [],
    'ExpressiveIterationEffect': [],
    'ExileGraveyardAllPlayersEffect': [],
    'AddExtraTurnControllerEffect': [],
    'ExileSpellEffect': [],
    'RemoveUpToAmountCountersEffect': [('amount', 'int')],
    # (first, second): damage to the targets tagged 1 and 2, the ability's first and second targets
    'DamageTargetAndTargetEffect': [('first', 'int'), ('second', 'int')],
    'TapTargetEffect': [],
    'CantAttackAloneSourceEffect': [],
    'MayTapOrUntapTargetEffect': [],
    'UntapTargetEffect': [(None, 'string')],
    'TapSourceEffect': [(None, 'bool')],
    'CounterUnlessPaysEffect': [('cost', 'cost'), ('exile', 'bool')],
    'DrawDiscardControllerEffect': [('draw', 'int'), ('discard', 'int'), ('optional', 'bool')],
    'DiscardControllerEffect': [('amount', 'int'), (None, 'false')],
    'DiscardCardYouChooseTargetEffect': [('filter', 'filter')],
    'ExileUntilSourceLeavesEffect': [],
    'ExploreTargetEffect': [(None, 'bool')],
    'MillCardsTargetEffect': [('amount', 'int')],
    'CantBeBlockedTargetEffect': [('duration', 'duration')],
    'AttachEffect': [(None, 'outcome'), (None, 'string')],
    'BoostEnchantedEffect': [('power', 'int'), ('toughness', 'int'), (None, 'while_on_battlefield')],
    'GainAbilityAttachedEffect': [('ability', 'ability'), (None, 'aura')],
    'CantAttackBlockAttachedEffect': [(None, 'aura')],
    'FightTargetsEffect': [],
    'FightEnchantedTargetEffect': [],
    'CantBeBlockedByMoreThanOneAttachedEffect': [(None, 'aura')],
    'CantCastMoreThanOneSpellEffect': [('who', 'target_controller')],
    'CantCastDuringYourTurnEffect': [],
    # cheaper equip abilities: the engine has no equipment, so it never applies
    'ReduceCostEquipTargetSourceEffect': [(None, 'int')],
    'PermanentsEnterBattlefieldTappedEffect': [('filter', 'filter')],
    'CantGainLifeAllEffect': [],
    'ReturnSourceFromGraveyardToBattlefieldEffect': [(None, 'false'), (None, 'false')],
    'GainPlusOneLifeReplacementEffect': [],
    'ReturnToHandFromBattlefieldAllEffect': [('filter', 'filter')],
    'CantBeCounteredControlledEffect': [('filter', 'filter'), ('duration', 'duration')],
    'DontCauseTriggerEffect': [],
    'SpellsCostIncreasingAllEffect': [('amount', 'int'), ('filter', 'filter'), ('who', 'target_controller')],
    'SetBasePowerToughnessPlusOneSourceEffect': [('amount', 'amount')],
    'LoseLifeOpponentsYouGainLifeLostEffect': [('amount', 'amount'), (None, 'string')],
    'DestroyAllNamedPermanentsEffect': [],
    'DoubleCountersSourceEffect': [('name', 'counter_type')],
    'DamageCantBePreventedEffect': [('duration', 'duration')],
    'ReturnFromGraveyardToBattlefieldTargetEffect': [('tapped', 'bool')],
    'DiscardTargetEffect': [('amount', 'int'), (None, 'false')],
    'SacrificeEffect': [('filter', 'filter'), ('amount', 'int'), (None, 'string')],
    'SacrificeOpponentsEffect': [('filter', 'filter')],
    'SacrificeAllEffect': [('amount', 'int'), ('filter', 'filter')],
    'GainFlashbackTargetEffect': [],
    'DistributeCountersEffect': [],
    'DamageMultiEffect': [],
    'CaseOfTheUneatenFeastEffect': [],
    'BoostEquippedEffect': [('power', 'int'), ('toughness', 'int')],
    'InfoEffect': [(None, 'string')],
    'BecomesCreatureAttachedEffect': [('token', 'creature_token'), (None, 'string'), (None, 'duration')],
    'PutOnLibraryTargetEffect': [('top', 'bool')],
    'BecomesAllBasicsControlledEffect': [(None, 'duration')],
    'CantAttackBlockUnlessConditionSourceEffect': [('condition', 'condition')],
    'CantBeBlockedByCreaturesSourceEffect': [('filter', 'filter'), ('duration', 'duration')],
    'MayCastTargetCardEffect': [('any_mana', 'cast_adjustment'), (None, 'false')],
    'CantBeBlockedByCreaturesWithLessPowerEffect': [],
    'MillCardsControllerEffect': [('amount', 'int')],
    'DiscardHandControllerEffect': [],
    'CantBeCounteredSourceEffect': [],
    'DamageControllerEffect': [('amount', 'int'), (None, 'string')],
    'DrawCardTargetEffect': [('amount', 'int')],
    'GetEnergyCountersControllerEffect': [('amount', 'amount')],
    'ScryEffect': [('amount', 'int'), (None, 'bool')],
    'SurveilEffect': [('amount', 'int'), (None, 'bool')],
    'DiscardAndDrawThatManyEffect': [('amount', 'int')],
    # subtypes aren't in the fingerprint and no rule here reads them yet
    'AddCardSubTypeSourceEffect': [(None, 'duration'), (None, 'bool'), ('subtype', 'subtype')],
}

COUNTS = {'CreaturesYouControlCount.PLURAL': 'creatures_you_control',
          'CreaturesYouControlCount.SINGULAR': 'creatures_you_control',
          'LandsYouControlCount.instance': 'lands_you_control', 'ManaSpentToCastCount.instance': 'mana_spent',
          'UnlicensedHearseValue.instance': 'exiled_with_source'}
PUT_CARDS = {'HAND', 'GRAVEYARD', 'BATTLEFIELD', 'BATTLEFIELD_TAPPED', 'EXILED', 'TOP_ANY', 'BOTTOM_ANY', 'BOTTOM_RANDOM'}

# classes a card file declares for itself, written by hand as the engine classes that do the same
CUSTOM = {
    'PithingNeedleEffect': lambda: {'class': 'PithingNeedleEffect'},
    'EmryLurkerOfTheLochPlayEffect': lambda: {'class': 'MayCastTargetFromGraveyardThisTurnEffect'},
    'PrismaticEndingEffect': lambda: {'class': 'PrismaticEndingEffect'},
    'BladeOfTheBloodchiefEffect': lambda: {'class': 'BladeOfTheBloodchiefEffect'},
    'EdificeOfAuthorityEffect': lambda: {'class': 'EdificeOfAuthorityEffect'},
    'BountyOfTheLuxaEffect': lambda: {'class': 'BountyOfTheLuxaEffect'},
    'BelieveEffect': lambda: {'class': 'BelieveEffect'},
    'NissasEncouragementEffect': lambda: {'class': 'NissasEncouragementEffect'},
    'HazoretsFavorSacrificeEffect': lambda: {'class': 'SacrificeTargetNextEndStepEffect'},
    'AidFromTheCowlEffect': lambda: {'class': 'AidFromTheCowlEffect'},
    'NissaNaturesArtisanEffect': lambda: {'class': 'NissaNaturesArtisanEffect'},
    'OraclesVaultFreeEffect': lambda: {'class': 'OraclesVaultFreeEffect'},
    'MerchantsDockhandEffect': lambda: {'class': 'MerchantsDockhandEffect'},
    'TreasureKeeperEffect': lambda: {'class': 'TreasureKeeperEffect'},
    'ShiftingWoodlandCopyEffect': lambda: {'class': 'ShiftingWoodlandCopyEffect'},
    'EmrakulThePromisedEndGainControlEffect': lambda: {'class': 'EmrakulThePromisedEndGainControlEffect'},
    'GemstoneCavernsAbility': lambda: {'class': 'GemstoneCavernsAbility', 'java': 'GemstoneCavernsAbility'},
    'UginsLabyrinthExileEffect': lambda: {'class': 'UginsLabyrinthExileEffect'},
    'UginsLabyrinthReturnEffect': lambda: {'class': 'UginsLabyrinthReturnEffect'},
    'RelicOfProgenitusEffect': lambda: {'class': 'RelicOfProgenitusEffect'},
    'KetramoseTriggeredAbility': lambda: {'class': 'TriggeredAbility', 'java': 'KetramoseTriggeredAbility', 'optional': False,
                                          'on': {'e': 'exiled_from_graveyard_or_battlefield_batch'},
                                          'effects': [{'class': 'DrawCardSourceControllerEffect', 'amount': 1},
                                                      {'class': 'LoseLifeSourceControllerEffect', 'amount': 1}]},
    'ExtinctionEventEffect': lambda: {'class': 'ExtinctionEventEffect'},
    'FumigateEffect': lambda: {'class': 'FumigateEffect'},
    # "enters with a +1/+1 counter on it for each Zombie card in your graveyard"
    # +1/+1 for each other creature named Relentless Rats (any controller's)
    'RelentlessRatsEffect': lambda: {'class': 'BoostSourceEffect', 'duration': 'WhileOnBattlefield',
                                     'power': {'v': 'plus', 'n': -1, 'of': {'v': 'permanents', 'filter': [
                                         {'p': 'type', 't': 'CREATURE'}, {'p': 'name', 'n': 'Relentless Rats'}]}},
                                     'toughness': {'v': 'plus', 'n': -1, 'of': {'v': 'permanents', 'filter': [
                                         {'p': 'type', 't': 'CREATURE'}, {'p': 'name', 'n': 'Relentless Rats'}]}}},
    'DiregrafColossusEffect': lambda: {'class': 'AddCountersSourceEffect', 'counter': {'name': '+1/+1', 'amount': 1},
                                       'amount': {'v': 'cards_in_graveyard', 'filter': [{'p': 'subtype', 's': 'Zombie'}]}},
    'GarrukCurseBreakerEffect': lambda: {'class': 'GarrukCurseBreakerEffect'},
    'PinnacleStarcageExileEffect': lambda: {'class': 'PinnacleStarcageExileEffect'},
    # a quest counter on it, then a reflexive trigger
    'EarthbenderAscensionEffect': lambda: {
        'class': 'AddCounterSourceReflexiveEffect', 'counter': {'name': 'quest', 'amount': 1},
        'rule': 'When you do, if it has four or more quest counters on it, put a +1/+1 counter on target creature '
                'you control. It gains trample until end of turn.',
        'condition': {'c': 'source_counter', 'name': 'quest', 'n': 4},
        'targets': [{'class': 'TargetPermanent', 'filter': [{'p': 'type', 't': 'CREATURE'}, {'p': 'controller', 'who': 'you'}]}],
        'effects': [{'class': 'AddCountersTargetEffect', 'counter': {'name': '+1/+1', 'amount': 1}},
                    {'class': 'GainAbilityTargetEffect', 'ability': {'class': 'TrampleAbility'}, 'duration': 'EndOfTurn'}]},
    'PinnacleStarcageTokenEffect': lambda: {'class': 'PinnacleStarcageTokenEffect', 'token': token('RobotToken')},
    # combat damage can't be prevented: the engine prevents no damage
    'QuestingBeastPreventionEffect': lambda: {'class': 'DamageCantBePreventedEffect', 'duration': 'WhileOnBattlefield'},
    'BoomerangBasicsEffect': lambda: {'class': 'BoomerangBasicsEffect'},
    'MagebaneLizardEffect': lambda: {'class': 'MagebaneLizardEffect'},
    'NarsetParterOfVeilsEffect': lambda: {'class': 'NarsetParterOfVeilsEffect'},
    # each opponent can cast spells only any time they could cast a sorcery
    'TeferiTimeRavelerReplacementEffect': lambda: {'class': 'TeferiTimeRavelerEffect'},
    # return target legendary creature card to the battlefield with haste; exile it at the next end step
    'GoryosVengeanceEffect': lambda: {'class': 'GoryosVengeanceEffect'},
    # reveal the top ten; a card of each card type into the hand, the rest on the bottom in a random order
    'AtraxaGrandUnifierEffect': lambda: {'class': 'AtraxaGrandUnifierEffect'},
    'ScreamingNemesisEffect': lambda: {'class': 'ScreamingNemesisEffect'},
    'GraveyardTrespasserEffect': lambda: {'class': 'GraveyardTrespasserEffect'},
    'MightformHarmonizerEffect': lambda: {'class': 'MightformHarmonizerEffect'},
    'GhostVacuumEffect': lambda: {'class': 'GhostVacuumEffect'},
    'ReflectionOfKikiJikiEffect': lambda: {'class': 'ReflectionOfKikiJikiEffect'},
    'QuantumRiddlerReplacementEffect': lambda: {'class': 'QuantumRiddlerReplacementEffect'},
    'TheScarabGodExileEffect': lambda: {'class': 'TheScarabGodExileEffect'},
    'ArtistsTalentEffect': lambda: {'class': 'ArtistsTalentEffect'},
    'LeylineOfTheVoidEffect': lambda: {'class': 'LeylineOfTheVoidEffect'},
    'ValgavothTerrorEaterReplacementEffect': lambda: {'class': 'ValgavothTerrorEaterReplacementEffect'},
    'LairOfTheHydraEffect': lambda: {'class': 'LairOfTheHydraEffect'},
    'LeylineOfTheGuildpactEffect': lambda: {'class': 'LeylineOfTheGuildpactEffect'},
    'MaraudingRaptorEffect': lambda: {'class': 'MaraudingRaptorEffect'},
    'ValgavothTerrorEaterPlayEffect': lambda: {'class': 'ValgavothTerrorEaterPlayEffect'},
    'TishanasTidebinderEffect': lambda: {'class': 'TishanasTidebinderEffect'},
    'CruelclawsHeistEffect': lambda: {'class': 'CruelclawsHeistEffect'},
    'LaughingJasperFlintEffect': lambda: {'class': 'LaughingJasperFlintEffect'},
    'RagavanNimblePilfererEffect': lambda: {'class': 'RagavanNimblePilfererEffect'},
    'SearingBlazeEffect': lambda: {'class': 'SearingBlazeEffect'},
    'RangerCaptainOfEosEffect': lambda: {'class': 'RangerCaptainOfEosEffect'},
    'TideShaperEffect': lambda: {'class': 'TideShaperEffect'},
    'AetherVialEffect': lambda: {'class': 'AetherVialEffect'},
    'SearingBlazeTarget': lambda: {'class': 'TargetPermanent', 'filter': [{'p': 'type', 't': 'CREATURE'},
                                                                          {'p': 'controlled_by_first_target'}]},
    'UnholyAnnexDrainEffect': lambda: {'class': 'UnholyAnnexDrainEffect'},
    'BecomePreparedSourceEffect': lambda: {'class': 'BecomePreparedSourceEffect'},
    'EsperOriginsEffect': lambda: {'class': 'EsperOriginsEffect'},
    'SummonEsperMaduinEffect': lambda: {'class': 'SummonEsperMaduinEffect'},
    'PetrifiedHamletCostEffect': lambda: {'class': 'PetrifiedHamletCostEffect'},
    'RottenmouthViperCostReductionEffect': lambda: {'class': 'RottenmouthViperCostReductionEffect'},
    'CacheGrabEffect': lambda: {'class': 'CacheGrabEffect', 'token': token('FoodToken')},
    'ScavengersTalentEffect': lambda: {'class': 'ScavengersTalentEffect'},
    'GhaltaPrimalHungerCostReductionEffect': lambda: {
        'class': 'SpellCostReductionForEachSourceEffect', 'per': 1,
        'amount': {'v': 'total_power', 'filter': [{'p': 'type', 't': 'CREATURE'}, {'p': 'controller', 'who': 'you'}]}},
    'OldGrowthTrollReturnEffect': lambda: {'class': 'OldGrowthTrollReturnEffect', 'token': token('TrollWarriorToken')},
    'AcademicDisputeEffect': lambda: {'class': 'AcademicDisputeEffect'},
    'LearnEffect': lambda: {'class': 'LearnEffect'},
    'AvenInterrupterEffect': lambda: {'class': 'AvenInterrupterEffect'},
    'CavernOfSoulsCantCounterEffect': lambda: {'class': 'CavernOfSoulsCantCounterEffect'},
    'GiadaFontOfHopeEntersBattlefieldEffect': lambda: {'class': 'GiadaFontOfHopeEntersBattlefieldEffect'},
    'SpellQuellerEntersEffect': lambda: {'class': 'SpellQuellerEntersEffect'},
    'ChandraTorchOfDefianceEffect': lambda: {'class': 'ChandraTorchOfDefianceEffect'},
    'SpellQuellerLeavesEffect': lambda: {'class': 'SpellQuellerLeavesEffect'},
    'ReturnSourceToHandNextEndStepEffect': lambda: {'class': 'ReturnSourceToHandNextEndStepEffect'},
    'MultiversalPassagePassageEffect': lambda: {'class': 'MultiversalPassagePassageEffect'},
    'LivelyDirgeEffect': lambda: {'class': 'LivelyDirgeEffect'},
    'EtchingOfKumanoReplacementEffect': lambda: {'class': 'EtchingOfKumanoReplacementEffect'},
    'ExileSagaAndReturnTransformedEffect': lambda: {'class': 'ExileSagaAndReturnTransformedEffect'},
    'TenaciousUnderdogEffect': lambda: {'class': 'TenaciousUnderdogEffect'},
    'NowhereToRunHexproofEffect': lambda: {'class': 'NowhereToRunHexproofEffect'},
    'NowhereToRunWardEffect': lambda: {'class': 'NowhereToRunWardEffect'},
    'EvolvedSleeperClericEffect': lambda: {'class': 'EvolvedSleeperClericEffect'},
    'EvolvedSleeperPhyrexianEffect': lambda: {'class': 'EvolvedSleeperPhyrexianEffect'},
    'GraveyardGluttonEffect': lambda: {'class': 'GraveyardTrespasserEffect'},
    # exile up to one target card from a graveyard; a creature card: create a Clue
    'RavenEagleEffect': lambda: {'class': 'RavenEagleEffect', 'then': {
        'class': 'CreateTokenEffect', 'token': token('ClueArtifactToken'), 'amount': 1}},
    # a 2/2 Cadet that gains haste until end of turn
    'StingerquillCharmEffect': lambda: {'class': 'CreateTokenEffect', 'token': token('CadetToken'), 'amount': 1,
                                        'haste_until_end_of_turn': True},
    'ProphesiedEndEffect': lambda: {'class': 'ProphesiedEndEffect'},
    'SoulGuideLanternEffect': lambda: {'class': 'SoulGuideLanternEffect'},
    'IngrisStingerquillEffect': lambda: {'class': 'IngrisStingerquillEffect'},
    'GevScaledScorchEntersEffect': lambda: {'class': 'GevScaledScorchEntersEffect'},
    'CantStayAwayEffect': lambda: {'class': 'CantStayAwayEffect'},
    'ClarionConquerorEffect': lambda: {'class': 'ClarionConquerorEffect'},
    'SunspineLynxEffect': lambda: {'class': 'SunspineLynxEffect'},
    'LovestruckBeastEffect': lambda: {'class': 'LovestruckBeastEffect'},
    # two 1/1 Spirits that gain haste until end of turn
    'SokenzanCrucibleOfDefianceEffect': lambda: {'class': 'CreateTokenEffect', 'token': token('SpiritToken'), 'amount': 2,
                                                 'haste_until_end_of_turn': True},
    'StrategicBetrayalEffect': lambda: {'class': 'StrategicBetrayalEffect'},
    'InvokeDespairEffect': lambda: {'class': 'InvokeDespairEffect'},
    'ChartACourseEffect': lambda: {'class': 'ChartACourseEffect'},
    'TemporaryLockdownEffect': lambda: {'class': 'TemporaryLockdownEffect'},
    'FeedTheSwarmEffect': lambda: {'class': 'FeedTheSwarmEffect'},
    'RedcapMeleeEffect': lambda: {'class': 'RedcapMeleeEffect'},
    'RamThroughEffect': lambda: {'class': 'RamThroughEffect'},
    'CullingRitualEffect': lambda: {'class': 'CullingRitualEffect'},
    'PlayWithFireEffect': lambda: {'class': 'PlayWithFireEffect'},
    # destroy target creature with mana value 2 or less (4 or less with revolt)
    # the target card in a graveyard goes to its owner's hand
    'FabledPassageSearchForLandEffect': lambda: {'class': 'FabledPassageSearchForLandEffect',
                                                 'filter': [LAND, {'p': 'supertype', 's': 'BASIC'}]},
    'FabledPassageUntapLandEffect': lambda: {'class': 'FabledPassageUntapLandEffect'},
    'FloodpitsDrownerEffect': lambda: {'class': 'FloodpitsDrownerEffect'},
    'ElvenPassageSearchForLandEffect': lambda: {'class': 'ElvenPassageEffect'},
    'RighteousValkyrieEffect': lambda: {'class': 'RighteousValkyrieEffect'},
    # look at the top seven, put up to X artifact and/or creature cards with mana value 3 or less onto the battlefield
    'KaylasReconstructionEffect': lambda: {
        'class': 'LookLibraryAndPickControllerEffect', 'look': 7, 'pick': 0, 'pick_x': True, 'up_to': True,
        'filter': [{'p': 'or', 'xs': [type_pred('ARTIFACT'), type_pred('CREATURE')]},
                   {'p': 'mv', 'cmp': 'FEWER_THAN', 'n': 4}],
        'picked': 'BATTLEFIELD', 'looked': 'BOTTOM_RANDOM'},
    'MagmaticHellkiteEffect': lambda: {'class': 'MagmaticHellkiteEffect'},
    'KothTheGeomancerEffect': lambda: {'class': 'KothTheGeomancerEffect'},
    'TheWanderingEmperorEffect': lambda: {'class': 'TheWanderingEmperorEffect'},
    'ValleyFlamecallerEffect': lambda: {'class': 'ValleyFlamecallerEffect'},
    'SoulScarMageDamageReplacementEffect': lambda: {'class': 'SoulScarMageDamageReplacementEffect'},
    'LoseGainEffect': lambda: {'class': 'LoseGainEffect'},
    'ScavengingOozeEffect': lambda: {'class': 'ScavengingOozeEffect'},
    'ReturnFromGraveyardToHandTargetEffect': lambda: {'class': 'ReturnToHandTargetEffect'},
    'FatalPushEffect': lambda: {'class': 'DestroyTargetIfManaValueEffect', 'n': 2, 'revolt_n': 4},
    # create a 2/2 Robot token tapped and attacking; sacrifice it at end of combat
    'KavaronHarrierEffect': lambda: {'class': 'CreateTokenEffect', 'token': token('RobotToken'), 'amount': 1,
                                     'tapped_and_attacking': True, 'sacrifice_at_end_of_combat': True},
    # whenever another creature you control enters (the "controlled" filter adds "you control")
    'EntersBattlefieldControlledTriggeredAbility': None,
    # whenever one or more opponents are dealt noncombat damage, creatures you control get +1/+0 until end of turn
    # "whenever a player casts a spell with mana value 3 or less, this deals 2 damage to that player"
    'EidolonOfTheGreatRevelTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'EidolonOfTheGreatRevelTriggeredAbility',
        'on': {'e': 'spell_cast', 'filter': [{'p': 'mv', 'cmp': 'OR_LESS', 'n': 3}], 'who': 'any'},
        'effects': [{'class': 'DamageTargetEffect', 'amount': 2}], 'set_target_pointer_player': True},
    # "whenever a creature an opponent controls dies, that player loses 2 life"
    'MassacreWurmTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'MassacreWurmTriggeredAbility',
        'on': {'e': 'dies_all', 'filter': [CREATURE, {'p': 'controller', 'who': 'opponent'}]},
        'effects': [{'class': 'LoseLifeTargetEffect', 'amount': 2}], 'set_target_pointer_player': True},
    # whenever another creature you control enters or dies, if its power is greater than this one's, put a +1/+1
    # counter on this
    'PeltCollectorTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'PeltCollectorTriggeredAbility', 'on': {'e': 'pelt_collector'},
        'effects': [{'class': 'AddCountersSourceEffect', 'counter': {'name': '+1/+1', 'amount': 1}}]},
    # when this dies, if it was a creature, return it to the battlefield; it's an enchantment
    'EnduringGlimmerTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'EnduringGlimmerTriggeredAbility', 'on': {'e': 'dies_if_creature'},
        'effects': [{'class': 'EnduringGlimmerReturnEffect'}]},
    'ExileTargetForSourceEffect': lambda: {'class': 'ExileTargetForSourceEffect'},
    'EliteSpellbinderEffect': lambda: {'class': 'EliteSpellbinderEffect'},
    'KitesailFreebooterExileEffect': lambda: {'class': 'KitesailFreebooterExileEffect', 'filter': [
        {'p': 'not', 'x': {'p': 'type', 't': 'LAND'}}, {'p': 'not', 'x': {'p': 'type', 't': 'CREATURE'}}]},
    'WanShiTongLibrarianTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'WanShiTongLibrarianTriggeredAbility', 'on': {'e': 'opponent_searches_library'},
        'effects': [{'class': 'AddCountersSourceEffect', 'counter': {'name': '+1/+1', 'amount': 1}},
                    {'class': 'DrawCardSourceControllerEffect', 'amount': 1}]},
    'SurrakElusiveHunterTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'SurrakElusiveHunterTriggeredAbility', 'on': {'e': 'surrak_targeted'},
        'effects': [{'class': 'DrawCardSourceControllerEffect', 'amount': 1}]},
    # for each opponent, a 1/1 Human tapped and attacking them
    'EnlightenedConfidantEffect': lambda: {'class': 'EnlightenedConfidantEffect'},
    'KraulHarpoonerEffect': lambda: {'class': 'KraulHarpoonerEffect'},
    'GreasefangOkibaBossEffect': lambda: {'class': 'GreasefangOkibaBossEffect'},
    'WeatheredRunestoneEffect': lambda: {'class': 'WeatheredRunestoneEffect'},
    'WeatheredRunestoneEffect2': lambda: {'class': 'WeatheredRunestoneEffect2'},
    'DauntlessBodyguardGainAbilityEffect': lambda: {'class': 'ChosenCreatureGainsAbilityEffect',
                                                    'ability': {'class': 'IndestructibleAbility'}},
    'SharkTyphoonCastEffect': lambda: {'class': 'CreateXXTokenEffect', 'token': token('SharkToken'), 'amount': {'v': 'x'}},
    'SharkTyphoonCycleEffect': lambda: {'class': 'CreateXXTokenEffect', 'token': token('SharkToken'), 'amount': {'v': 'x'}},
    'AdelineResplendentCatharEffect': lambda: {'class': 'CreateTokenEffect', 'token': token('HumanToken'), 'amount': 1,
                                               'tapped_and_attacking': True},
    # "whenever you gain life for the first time during each of your turns, create a 1/1 white Cat"
    'CatCollectorTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'CatCollectorTriggeredAbility', 'on': {'e': 'you_gain_life_first_time'},
        'trigger_condition': {'c': 'my_turn'}, 'optional': False,
        'effects': [{'class': 'CreateTokenEffect', 'token': token('Cat11Token'), 'amount': 1}]},
    # "whenever you put one or more +1/+1 counters on this creature, draw a card. This ability triggers only once each turn."
    'ExemplarOfLightTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'ExemplarOfLightTriggeredAbility',
        'on': {'e': 'counters_added_source', 'counter': '+1/+1'}, 'optional': False, 'limit_per_turn': 1,
        'effects': [{'class': 'DrawCardSourceControllerEffect', 'amount': 1}]},
    # "when there are four or more page counters on this, exile it. If you do, you gain 4 life."
    'MazemindTomeTriggeredAbility': lambda: {
        'class': 'TriggeredAbility', 'java': 'MazemindTomeTriggeredAbility',
        'on': {'e': 'source_counters_at_least', 'counter': 'page', 'n': 4}, 'optional': False,
        'effects': [{'class': 'DoIfCostPaid', 'effect': {'class': 'GainLifeEffect', 'amount': 4},
                     'cost': [{'class': 'ExileSourceCost'}], 'optional': False}]},
    # "if this land isn't a creature, it becomes a 2/4 Wizard creature with 'Whenever you cast an instant or sorcery
    # spell, this creature gets +1/+0 until end of turn.' It's still a land."
    'GreatHallOfTheBiblioplexEffect': lambda: {'class': 'GreatHallOfTheBiblioplexEffect', 'token': {
        'name': '', 'types': ['CREATURE'], 'colors': [], 'subtypes': ['Wizard'], 'power': 2, 'toughness': 4,
        'abilities': [{'class': 'TriggeredAbility', 'on': {'e': 'spell_cast', 'who': 'you', 'filter': [
            {'p': 'or', 'xs': [type_pred('INSTANT'), type_pred('SORCERY')]}]}, 'optional': False,
            'effects': [{'class': 'BoostSourceEffect', 'power': 1, 'toughness': 0, 'duration': 'EndOfTurn'}],
            'rule': 'Whenever you cast an instant or sorcery spell, {this} gets +1/+0 until end of turn.'}]}},
    'LilianaDreadhordeGeneralEffect': lambda: {'class': 'LilianaDreadhordeGeneralEffect'},
    'TwinflameTyrantEffect': lambda: {'class': 'TwinflameTyrantEffect'},
    'WitchsOvenEffect': lambda: {'class': 'WitchsOvenEffect', 'token': token('FoodToken')},
    'MasterOfBarbsTriggeredAbility': lambda: {
        'class': 'DamagedOpponentsNoncombatTriggeredAbility', 'java': 'MasterOfBarbsTriggeredAbility',
        'effects': [{'class': 'BoostControlledEffect', 'power': 1, 'toughness': 0, 'duration': 'EndOfTurn',
                     'filter': [CREATURE], 'exclude_source': False}]},
}

# abilities with arguments: class -> (field name, kind) per argument; optional trailing arguments may be left out
ABILITIES = {
    'EntersBattlefieldTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'CastSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'CrewsVehicleSourceTriggeredAbility': [('effect', 'effect')],
    'BecomesTappedSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'AttacksTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'), (None, 'string')],
    'SimpleStaticAbility': [('effect', 'effect')],
    'AsEntersBattlefieldAbility': [('effect', 'effect'), (None, 'string')],
    'EntersBattlefieldTappedAbility': [],
    'KickerAbility': [('cost', 'cost_text')],
    'BlightAbility': [('amount', 'int')],
    'FlashbackAbility': [(None, 'this'), ('cost', 'mana_costs')],
    'HarmonizeAbility': [(None, 'this'), ('cost', 'cost_text')],
    'EscapeAbility': [(None, 'this'), ('cost', 'cost_text'), ('exile', 'int')],
    'EscapesWithAbility': [('counters', 'int')],
    'SpectacleAbility': [(None, 'this'), ('cost', 'mana_costs')],
    # cast for the cleave cost instead, with the effect without the bracketed words
    'CleaveAbility': [(None, 'this'), ('effect', 'effect'), ('cost', 'cost_text')],
    'WarpAbility': [(None, 'this'), ('cost', 'cost_text')],
    # enters with an effect applied, if the condition holds (a replacement effect, not a trigger)
    'EntersBattlefieldAbility': [('effect', 'effect'), ('condition', 'condition'), (None, 'string'), (None, 'string')],
    'EntersBattlefieldAllTriggeredAbility': [('effect', 'effect'), ('filter', 'filter'), ('optional', 'bool')],
    'LandfallAbility': [('effect', 'effect'), ('optional', 'bool')],
    'DiesSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'PutIntoGraveFromBattlefieldSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'ActivateAsSorceryActivatedAbility': [('effect', 'effect'), ('cost', 'cost')],
    'SimpleActivatedAbility': [('effect', 'effect'), ('cost', 'cost')],
    # once each game, for its mana cost less the turn this entered
    'PowerUpAbility': [('effect', 'effect'), ('cost', 'cost')],
    'ProwessAbility': [],
    'EntersPreparedAbility': [],
    # the step triggers take a TargetController first; the shorter overloads mean "your" (filled in by `ability`)
    'BeginningOfUpkeepTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'),
                                          ('optional', 'bool')],
    'BeginningOfCombatTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'),
                                          ('optional', 'bool')],
    'BeginningOfEndStepTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'),
                                           ('optional', 'bool'), ('condition', 'condition')],
    'BeginningOfDrawTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'), ('optional', 'bool')],
    'BeginningOfPostcombatMainTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'), ('optional', 'bool')],
    'BeginningOfSecondMainTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'), ('optional', 'bool')],
    'BeginningOfFirstMainTriggeredAbility': [('controller', 'target_controller'), ('effect', 'effect'), ('optional', 'bool')],
    'GainLifeControllerTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'), (None, 'bool'),
                                           (None, 'false')],
    'GainLifeFirstTimeTriggeredAbility': [('effect', 'effect')],
    'DealsCombatDamageToAPlayerTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'),
                                                   ('set_target_pointer', 'bool')],
    'EntersBattlefieldOrDiesSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'), (None, 'bool')],
    'EntersBattlefieldOrAttacksSourceTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'TransformsOrEntersTriggeredAbility': [('effect', 'effect'), ('optional', 'bool')],
    'DiesThisOrAnotherTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'), ('filter', 'filter')],
    'SpellCastControllerTriggeredAbility': [('effect', 'effect'), ('filter', 'filter'), ('optional', 'bool'),
                                            (None, 'no_pointer')],
    'UnlockThisDoorTriggeredAbility': [('effect', 'effect'), ('optional', 'bool'), ('left', 'bool')],
}

COLOR_LETTERS = {'White': 'W', 'Blue': 'U', 'Black': 'B', 'Red': 'R', 'Green': 'G'}

STEP_TRIGGERS = {'BeginningOfUpkeepTriggeredAbility': 'upkeep', 'BeginningOfCombatTriggeredAbility': 'combat',
                 'BeginningOfEndStepTriggeredAbility': 'end', 'BeginningOfDrawTriggeredAbility': 'draw',
                 'BeginningOfPostcombatMainTriggeredAbility': 'main2',
                 # the second main phase of the turn: the postcombat one, as the engine's turns have one combat
                 'BeginningOfSecondMainTriggeredAbility': 'main2',
                 'BeginningOfFirstMainTriggeredAbility': 'main1', 'BeginningOfPreCombatMainTriggeredAbility': 'main1'}


def triggered(a):
    """A Java triggered-ability class, as read from its constructor, as the engine's one `TriggeredAbility`: what it
    triggers on, then the rest"""
    cls = a['class']
    if cls in STEP_TRIGGERS:
        on = {'e': 'step_begins', 'step': STEP_TRIGGERS[cls], 'who': a.pop('controller')}
    elif cls == 'UnlockThisDoorTriggeredAbility':
        on = {'e': 'door_unlocked', 'left': a.pop('left')}
    elif cls == 'ProwessAbility':
        on = {'e': 'spell_cast', 'filter': [{'p': 'not', 'x': CREATURE}], 'who': 'you'}
        a['effects'] = [{'class': 'BoostSourceEffect', 'power': 1, 'toughness': 1, 'duration': 'EndOfTurn'}]
    elif cls == 'LandfallAbility':
        on = {'e': 'enters_all', 'filter': [LAND, YOU]}
        # Koth's effect looks at the land that entered
        if any(e.get('class') == 'KothTheGeomancerEffect' for e in a.get('effects', [])):
            a['set_target_pointer'] = True
    elif cls == 'EntersBattlefieldAllTriggeredAbility':
        on = {'e': 'enters_all', 'filter': a.pop('filter')}
    elif cls == 'DiesThisOrAnotherTriggeredAbility':
        on = {'e': 'dies_all', 'filter': a.pop('filter', [CREATURE]), 'include_self': True}
    elif cls == 'SpellCastControllerTriggeredAbility':
        on = {'e': 'spell_cast', 'filter': a.pop('filter'), 'who': 'you'}
    elif cls in SIMPLE_TRIGGERS:
        on = {'e': SIMPLE_TRIGGERS[cls]}
    else:
        return a
    out = {'class': 'TriggeredAbility', 'java': a.pop('java', cls), 'on': on}
    out.update((k, v) for k, v in a.items() if k != 'class')
    return out


SIMPLE_TRIGGERS = {'EntersBattlefieldTriggeredAbility': 'enters', 'CrewsVehicleSourceTriggeredAbility': 'crews_vehicle',
                   'BecomesTappedSourceTriggeredAbility': 'becomes_tapped', 'CastSourceTriggeredAbility': 'cast_this', 'AttacksTriggeredAbility': 'attacks',
                   'DiesSourceTriggeredAbility': 'dies', 'PutIntoGraveFromBattlefieldSourceTriggeredAbility': 'dies',
                   'EntersBattlefieldOrDiesSourceTriggeredAbility':
                   'enters_or_dies', 'EntersBattlefieldOrAttacksSourceTriggeredAbility': 'enters_or_attacks',
                   'TransformsOrEntersTriggeredAbility': 'transforms_or_enters',
                   'GainLifeControllerTriggeredAbility': 'you_gain_life',
                   'GainLifeFirstTimeTriggeredAbility': 'you_gain_life_first_time',
                   'DealsCombatDamageToAPlayerTriggeredAbility': 'combat_damage_to_player',
                   'DamagedOpponentsNoncombatTriggeredAbility': 'opponents_dealt_noncombat_damage'}

TARGET_CONTROLLERS = {'TargetController.YOU': 'you', 'TargetController.NEXT': 'any', 'TargetController.ANY': 'any',
                      'TargetController.OPPONENT': 'opponent'}

COUNTERS = {'CounterType.P1P1': '+1/+1', 'CounterType.BRICK': 'brick', 'CounterType.VELOCITY': 'velocity', 'CounterType.PAGE': 'page', 'CounterType.BLIGHT': 'blight', 'CounterType.LOYALTY': 'loyalty', 'CounterType.STUN': 'stun',
            'CounterType.SHIELD': 'shield', 'CounterType.CHARGE': 'charge',
            'CounterType.M1M1': '-1/-1', 'CounterType.INDESTRUCTIBLE': 'indestructible'}
CONDITIONS = {'KickedCondition.ONCE': {'c': 'kicked'},
              'MetalcraftCondition.instance': {'c': 'permanents', 'filter': [{'p': 'type', 't': 'ARTIFACT'}, {'p': 'controller', 'who': 'you'}], 'cmp': 'OR_GREATER', 'n': 3}, 'PlayLandCondition.instance': {'c': 'played_land'}, 'DeliriumCondition.instance': {'c': 'delirium'},
              # a bargained spell is kicked (`BargainAbility`)
              'BargainedCondition.instance': {'c': 'kicked'},
              'ObsessivePursuitCondition.instance': {'c': 'sacrificed_this_turn', 'n': 3},
              'KaitoBaneOfNightmaresCondition.instance': {'c': 'and', 'xs': [
                  {'c': 'my_turn'}, {'c': 'source_counter', 'name': 'loyalty', 'n': 1}]},
              # blight is an optional additional cost the engine keeps as the spell's kicked flag
              'BlightedCondition.instance': {'c': 'kicked'}, 'TeamworkCondition.instance': {'c': 'kicked'}, 'RaidCondition.instance': {'c': 'raid'},
              'RevoltCondition.instance': {'c': 'revolt'},
              'SourceAttackingCondition.instance': {'c': 'source_attacking'},
              'AttackedOrBlockedThisCombatSourceCondition.instance': {'c': 'attacked_or_blocked_this_combat'},
              'OpponentsLostLifeCondition.instance': {'c': 'opponents_lost_life'},
              'OpponentHasMoreLifeCondition.instance': {'c': 'opponent_more_life'},
              'DrewTwoOrMoreCardsCondition.instance': {'c': 'drew_cards', 'n': 2},
              'OpponentHasMoreCardsInHandCondition.instance': {'c': 'opponent_more_cards'},
              'HellbentCondition.instance': {'c': 'cards_in_hand_at_most', 'n': 0},
              'HeckbentCondition.instance': {'c': 'cards_in_hand_at_most', 'n': 1},
              # spell mastery: two or more instant and/or sorcery cards in your graveyard
              'SpellMasteryCondition.instance': {'c': 'cards_in_graveyard', 'n': 2, 'filter': [
                  {'p': 'or', 'xs': [{'p': 'type', 't': 'INSTANT'}, {'p': 'type', 't': 'SORCERY'}]}]},
              'MyTurnCondition.instance': {'c': 'my_turn'},
              'MorbidCondition.instance': {'c': 'morbid'},
              'MomoFriendlyFlierCondition.instance': {'c': 'and', 'xs': [{'c': 'my_turn'}, {'c': 'not', 'x': {'c': 'flier_cast'}}]},
              'YouGainedLifeCondition.getZero()': {'c': 'you_gained_life'},
              'SolvedSourceCondition.SOLVED': {'c': 'solved'},
              'TwoOfManaColorSpentCondition.WHITE': {'c': 'mana_color_spent', 'color': 'W', 'n': 2},
              'TwoOfManaColorSpentCondition.BLUE': {'c': 'mana_color_spent', 'color': 'U', 'n': 2},
              'TwoOfManaColorSpentCondition.BLACK': {'c': 'mana_color_spent', 'color': 'B', 'n': 2},
              'TwoOfManaColorSpentCondition.RED': {'c': 'mana_color_spent', 'color': 'R', 'n': 2},
              'TwoOfManaColorSpentCondition.GREEN': {'c': 'mana_color_spent', 'color': 'G', 'n': 2},
              'SolvedSourceCondition.UNSOLVED': {'c': 'not', 'x': {'c': 'solved'}},
              'KeenEyedCuratorCondition.instance': {'c': 'exiled_card_types', 'n': 4},
              'FerociousCondition.instance': {'c': 'permanents', 'cmp': 'OR_GREATER', 'n': 1, 'filter': [
                  {'p': 'type', 't': 'CREATURE'}, {'p': 'power', 'cmp': 'OR_GREATER', 'n': 4}]},
              'SourceEnteredThisTurnCondition.TRUE': {'c': 'source_entered_this_turn'},
              'CastFromEverywhereSourceCondition.instance': {'c': 'cast_from_everywhere'},
              'NotMyTurnCondition.instance': {'c': 'not', 'x': {'c': 'my_turn'}},
              'StartingTownCondition.instance': {'c': 'starting_town'},
              'LessonsInGraveCondition.THREE': {'c': 'cards_in_graveyard', 'n': 3,
                                                 'filter': [{'p': 'subtype', 's': 'Lesson'}]}}
TOKENS = os.path.join(ROOT, 'Mage', 'src', 'main', 'java', 'mage', 'game', 'permanent', 'token')

# an emblem's static effect lasts the game: as long as its source is there, like a permanent's
DURATIONS = {'Duration.EndOfTurn': 'EndOfTurn', 'Duration.WhileOnBattlefield': 'WhileOnBattlefield',
             'Duration.EndOfGame': 'WhileOnBattlefield', 'Duration.UntilYourNextTurn': 'UntilYourNextTurn'}

STATIC_FILTERS = os.path.join(ROOT, 'Mage', 'src', 'main', 'java', 'mage', 'filter', 'StaticFilters.java')


def type_pred(t):
    return {'p': 'type', 't': t}


CREATURE, LAND, YOU, OPPONENT = type_pred('CREATURE'), type_pred('LAND'), {'p': 'controller', 'who': 'you'}, \
    {'p': 'controller', 'who': 'opponent'}

# counts of permanents you control, as filters
COUNT_FILTERS = {'CreaturesYouControlCount.PLURAL': [CREATURE, YOU], 'CreaturesYouControlCount.SINGULAR': [CREATURE, YOU],
                 'LandsYouControlCount.instance': [LAND, YOU],
                 'ArtifactYouControlCount.instance': [{'p': 'type', 't': 'ARTIFACT'}, YOU]}

# arguments a constructor overload leaves out, and what Java uses then
DEFAULTS = {'amount': 1, 'filter': [CREATURE], 'exclude_source': False, 'condition': None, 'duration': 'EndOfTurn'}

# filter classes: the predicates their constructors add (a SubType argument adds that subtype too)
FILTER_CLASSES = {
    'FilterAttackingCreature': [CREATURE, {'p': 'attacking'}],
    'FilterArtifactCreaturePermanent': [type_pred('ARTIFACT'), CREATURE],
    'FilterPermanent': [], 'FilterCreaturePermanent': [CREATURE], 'FilterControlledPermanent': [YOU],
    'FilterControlledCreaturePermanent': [CREATURE, YOU], 'FilterLandPermanent': [LAND],
    'FilterControlledLandPermanent': [LAND, YOU], 'FilterNonlandPermanent': [{'p': 'not', 'x': LAND}],
    'FilterArtifactPermanent': [type_pred('ARTIFACT')], 'FilterControlledArtifactPermanent': [type_pred('ARTIFACT'), YOU],
    'FilterEnchantmentPermanent': [type_pred('ENCHANTMENT')],
    'FilterArtifactOrEnchantmentPermanent': [{'p': 'or', 'xs': [type_pred('ARTIFACT'), type_pred('ENCHANTMENT')]}],
    'FilterControlledEnchantmentPermanent': [type_pred('ENCHANTMENT'), YOU],
    'FilterPlaneswalkerPermanent': [type_pred('PLANESWALKER')],
    'FilterControlledPlaneswalkerPermanent': [type_pred('PLANESWALKER'), YOU],
    'FilterCreatureOrPlaneswalkerPermanent': [{'p': 'or', 'xs': [CREATURE, type_pred('PLANESWALKER')]}],
    'FilterOpponentsCreaturePermanent': [CREATURE, OPPONENT],
    'FilterSpell': [], 'FilterCreatureSpell': [CREATURE], 'FilterNoncreatureSpell': [{'p': 'not', 'x': CREATURE}],
    'FilterInstantOrSorcerySpell': [{'p': 'or', 'xs': [type_pred('INSTANT'), type_pred('SORCERY')]}],
    'FilterCard': [], 'FilterObject': [], 'FilterOwnedCard': [{'p': 'owner', 'who': 'you'}], 'FilterCreatureCard': [CREATURE], 'FilterLandCard': [LAND], 'FilterArtifactCard': [{'p': 'type', 't': 'ARTIFACT'}],
    'FilterNonlandCard': [{'p': 'not', 'x': LAND}], 'FilterPermanentCard': [{'p': 'permanent_card'}],
    'FilterInstantOrSorceryCard': [{'p': 'or', 'xs': [type_pred('INSTANT'), type_pred('SORCERY')]}],
    'FilterBasicLandCard': [LAND, {'p': 'supertype', 's': 'BASIC'}],
    'FilterStackObject': [], 'FilterCreatureCards': [CREATURE],
}

COMPARISONS = {'FEWER_THAN', 'MORE_THAN', 'EQUAL_TO', 'OR_LESS', 'OR_GREATER'}
COLORS = {'WHITE': 'W', 'BLUE': 'U', 'BLACK': 'B', 'RED': 'R', 'GREEN': 'G'}
KEYWORD_CLASSES = {k: k for k in KEYWORDS}


def predicate(text, subtypes):
    """A Java predicate expression (canonical text) -> a predicate object"""
    text = text.strip()
    while text.startswith('(') and text.endswith(')') and split_args(text[1:-1]) == [text[1:-1].strip()]:
        text = text[1:-1].strip()
    if (m := re.fullmatch(r'CardType\.(\w+)\.getPredicate\(\)', text)):
        return type_pred(m.group(1))
    if (m := re.fullmatch(r'SubType\.(\w+)\.getPredicate\(\)', text)) and m.group(1) in subtypes:
        return {'p': 'subtype', 's': subtypes[m.group(1)]}
    if (m := re.fullmatch(r'SuperType\.(\w+)\.getPredicate\(\)', text)):
        return {'p': 'supertype', 's': m.group(1)}
    if (m := re.fullmatch(r'TargetController\.(YOU|OPPONENT|NOT_YOU)\.get(Controller|Owner)Predicate\(\)', text)):
        return {'p': m.group(2).lower(), 'who': 'you' if m.group(1) == 'YOU' else 'opponent'}
    if text == 'ChosenNamePredicate.instance':
        return {'p': 'chosen_name'}
    if text == 'AnotherPredicate.instance':
        return {'p': 'another'}
    if text == 'MulticoloredPredicate.instance':
        return {'p': 'multicolored'}
    if (m := re.fullmatch(r'new ManaValueEqualToCountersSourceCountPredicate\((CounterType\.\w+)\)', text)) and m.group(1) in COUNTERS:
        return {'p': 'mana_value_equal_source_counters', 'counter': COUNTERS[m.group(1)]}
    if text == 'UrzasSagaPredicate.instance':
        return {'p': 'mana_cost_in', 'costs': ['{0}', '{1}']}
    if text == 'ColorlessPredicate.instance':
        return {'p': 'colorless'}
    if text == 'new ManaSpentToCastPredicate(ComparisonType.EQUAL_TO,0)':
        return {'p': 'no_mana_spent'}
    # with another Aura attached to it
    if text == 'AuraAttachedPredicate.instance':
        return {'p': 'aura_attached'}
    if text == 'AdventurePredicate.instance':
        return {'p': 'adventure'}
    # two players: the defending player is the opponent
    if text == 'DefendingPlayerOwnsCardPredicate.instance':
        return {'p': 'owner', 'who': 'opponent'}
    if text == 'DefendingPlayerControlsSourceAttackingPredicate.instance':
        return {'p': 'controller', 'who': 'opponent'}
    # an outlaw: an Assassin, Mercenary, Pirate, Rogue or Warlock
    if text == 'CutDownPredicate.instance':
        return {'p': 'total_power_toughness', 'n': 5}
    # Death Baron: Skeletons, and other Zombies
    if text == 'DeathBaronPredicate.instance':
        return {'p': 'or', 'xs': [{'p': 'subtype', 's': subtypes['SKELETON']},
                                  {'p': 'and', 'xs': [{'p': 'another'}, {'p': 'subtype', 's': subtypes['ZOMBIE']}]}]}
    if text == 'OutlawPredicate.instance':
        return {'p': 'or', 'xs': [{'p': 'subtype', 's': t} for t in ('Assassin', 'Mercenary', 'Pirate', 'Rogue', 'Warlock')]}
    if (m := re.fullmatch(r'new (ManaValue|Power|Toughness)Predicate\(ComparisonType\.(\w+),(-?\d+)\)', text)) \
            and m.group(2) in COMPARISONS:
        return {'p': {'ManaValue': 'mv'}.get(m.group(1), m.group(1).lower()), 'cmp': m.group(2), 'n': int(m.group(3))}
    if (m := re.fullmatch(r'new ColorPredicate\(ObjectColor\.(\w+)\)', text)) and m.group(1) in COLORS:
        return {'p': 'color', 'c': COLORS[m.group(1)]}
    if (m := re.fullmatch(r'new CastFromZonePredicate\(Zone\.(GRAVEYARD|EXILED|HAND)\)', text)):
        return {'p': 'cast_from', 'zone': {'GRAVEYARD': 'graveyard', 'EXILED': 'exile', 'HAND': 'hand'}[m.group(1)]}
    if (m := re.fullmatch(r'TappedPredicate\.(TAPPED|UNTAPPED)', text)):
        return {'p': 'tapped', 'v': m.group(1) == 'TAPPED'}
    if text == 'AttackingPredicate.instance':
        return {'p': 'attacking'}
    if text == 'AttackingOrBlockingPredicate.instance':
        return {'p': 'in_combat'}
    if (m := re.fullmatch(r'TokenPredicate\.(TRUE|FALSE)', text)):
        return {'p': 'token', 'v': m.group(1) == 'TRUE'}
    if (m := re.fullmatch(r'(CounterType\.\w+)\.getPredicate\(\)', text)) and m.group(1) in COUNTERS:
        return {'p': 'counter', 'name': COUNTERS[m.group(1)]}
    if (m := re.fullmatch(r'new AbilityPredicate\((\w+)\.class\)', text)) and m.group(1) in KEYWORDS:
        return {'p': 'ability', 'a': {'class': m.group(1)}}
    if (m := re.fullmatch(r'new NamePredicate\("([^"]*)"\)', text)):
        return {'p': 'name', 'n': m.group(1)}
    if (m := re.fullmatch(r'Predicates\.not\((.*)\)', text)):
        return {'p': 'not', 'x': predicate(m.group(1), subtypes)}
    if (m := re.fullmatch(r'Predicates\.(or|and)\((.*)\)', text)):
        return {'p': m.group(1), 'xs': [predicate(x, subtypes) for x in split_args(m.group(2))]}
    raise Unsupported('predicate ' + text)


def filter_object(cls, args, adds, subtypes):
    """`new <cls>(args)` plus the predicates later `add`ed to it -> a filter (a list of predicates)"""
    if cls not in FILTER_CLASSES:
        raise Unsupported('filter class ' + cls)
    preds = [dict(p) for p in FILTER_CLASSES[cls]]
    for a in args:
        a = a.strip()
        if re.fullmatch(r'"[^"]*"', a):
            continue
        if (m := re.fullmatch(r'SubType\.(\w+)', a)) and m.group(1) in subtypes:
            preds.append({'p': 'subtype', 's': subtypes[m.group(1)]})
            continue
        raise Unsupported('filter argument ' + a)
    for x in adds:
        preds.append(predicate(x, subtypes))
    return preds


ANY_OTHER_TARGET = [{'p': 'any_other_target'}]
# any target other than the ability's first target (`AnotherTargetPredicate(2, true)` on both halves: Drakuseth)
ANY_TARGET_NOT_FIRST = [{'p': 'any_target_not_first'}]


def declared_filters(source, subtypes):
    """filter fields (`static final FilterX name = new FilterX(…); static { name.add(…); }`) -> filters by name; one
    that can't be read is left out (the card fails only if it uses it)"""
    out = {}
    # predicate fields (`static final Predicate<…> predicate = Predicates.or(…);`), by name
    preds = {m.group(1): canonical(m.group(2))
             for m in re.finditer(r'static final Predicate<[\w<>, ]*> (\w+)\s*=\s*(.*?);', source, re.S)}
    # top-level classes: a field's `add`s are read from its own class (two classes may both have a `filter`; the
    # first, the card's, wins)
    starts = [m.start() for m in re.finditer(r'^(?:public |final |abstract )*(?:class|enum) ', source, re.M)] + [len(source)]
    whole = source
    for m in re.finditer(r'static (?:final )?(?:\w+) (\w+)\s*=\s*new (Filter\w+)\((.*?)\);', whole, re.S):
        name, cls, args = m.group(1), m.group(2), split_args(canonical(m.group(3)))
        if name in out:
            continue
        source = whole[max([0] + [x for x in starts if x <= m.start()]):min(x for x in starts if x > m.start())]
        # "any other target": a creature, planeswalker, battle or player, not the source
        if cls == 'FilterAnyTarget' and re.search(r'\b' + name + r'\.getPermanentFilter\(\)\.add\(AnotherPredicate\.instance\);', source) \
                and not re.search(r'\b' + name + r'\.(add|getPlayerFilter)', source):
            out[name] = ANY_OTHER_TARGET
            continue
        if cls == 'FilterAnyTarget' \
                and re.search(r'\b' + name + r'\.getPlayerFilter\(\)\.add\(new AnotherTargetPredicate\(2,\s*true\)\);', source) \
                and re.search(r'\b' + name + r'\.getPermanentFilter\(\)\.add\(new AnotherTargetPredicate\(2,\s*true\)\);', source) \
                and len(re.findall(r'\b' + name + r'\.', source)) == 2:
            out[name] = ANY_TARGET_NOT_FIRST
            continue
        adds = [canonical(p) for p in re.findall(r'\b' + name + r'\.add\((.*?)\);', source, re.S)]
        adds = [preds.get(a, a) for a in adds]
        # "spell or creature" (Fatehold Charm): any spell, or a creature
        if cls == 'FilterSpellOrPermanent' and not adds \
                and re.search(r'\b' + name + r'\.setPermanentFilter\(new FilterCreaturePermanent\(\)\);', source):
            out[name] = SPELL_OR_CREATURE
            continue
        try:
            out[name] = filter_object(cls, args, adds, subtypes)
        except Unsupported as e:
            out[name] = e
    return out


_static_filters = None

# gifts: what's promised, and the token the opponent gets
GIFTS = {'GiftType.TAPPED_FISH': ('a tapped Fish', 'FishNoAbilityToken'), 'GiftType.CARD': ('a card', None)}

# a `FilterSpellOrPermanent` of any spell or a creature
SPELL_OR_CREATURE = ('spell or creature',)


def static_filters(subtypes):
    global _static_filters
    if _static_filters is None:
        _static_filters = declared_filters(open(STATIC_FILTERS).read(), subtypes)
    return _static_filters


# targets: class -> (what it targets, the filter its constructors use when none is given)
TARGET_CLASSES = {
    'TargetPermanent': ('TargetPermanent', []),
    'TargetCreaturePermanent': ('TargetPermanent', [CREATURE]),
    'TargetControlledCreaturePermanent': ('TargetPermanent', [CREATURE, YOU]),
    'TargetControlledPermanent': ('TargetPermanent', [YOU]),
    'TargetOpponentsCreaturePermanent': ('TargetPermanent', [CREATURE, OPPONENT]),
    'TargetNonlandPermanent': ('TargetPermanent', [{'p': 'not', 'x': LAND}]),
    'TargetArtifactPermanent': ('TargetPermanent', [type_pred('ARTIFACT')]),
    'TargetEnchantmentPermanent': ('TargetPermanent', [type_pred('ENCHANTMENT')]),
    'TargetLandPermanent': ('TargetPermanent', [LAND]),
    'TargetControlledLandPermanent': ('TargetPermanent', [LAND, YOU]),
    'TargetPlaneswalkerPermanent': ('TargetPermanent', [type_pred('PLANESWALKER')]),
    'TargetCreatureOrPlaneswalker': ('TargetPermanent', [{'p': 'or', 'xs': [CREATURE, type_pred('PLANESWALKER')]}]),
    'TargetAttackingCreature': ('TargetPermanent', [CREATURE, {'p': 'attacking'}]),
    'TargetAttackingOrBlockingCreature': ('TargetPermanent', [CREATURE, {'p': 'in_combat'}]),
    'TargetSpell': ('TargetSpell', []),
    'TargetActivatedOrTriggeredAbility': ('TargetStackAbility', []),
    'TargetCardInGraveyard': ('TargetCardInGraveyard', []),
    'TargetCardInYourGraveyard': ('TargetCardInGraveyard', [{'p': 'owner', 'who': 'you'}]),
    'TargetCardInOpponentsGraveyard': ('TargetCardInGraveyard', [{'p': 'owner', 'who': 'opponent'}]),
    'TargetCardInASingleGraveyard': ('TargetCardInGraveyard', []),
}
# targets of players (no arguments)
PLAYER_TARGETS = {'TargetAnyTarget', 'TargetPlayer', 'TargetOpponent', 'TargetPlayerOrPlaneswalker', 'TargetOpponentOrPlaneswalker'}

# chained calls that only change texts, hints or the AI's view
TEXT_SETTERS = {'setText', 'withShowSacText', 'setAbilityWord', 'withFlavorWord', 'addHint', 'concatBy', 'setTriggerPhrase', 'withChooseHint',
                'setOutcome', 'setRuleAtTheTop', 'withRuleTextReplacement', 'withDurationRuleAtStart', 'withTargetDescription',
                'withTextOptions', 'setTargetTag', 'setRuleVisible', 'setIdentifier'}

# abilities every card has, which the constructor doesn't add
IMPLICIT = {'SpellAbility', 'PlayLandAbility', 'AdventureCardSpellAbility', 'OmenCardSpellAbility', 'PrepareCardSpellAbility'}


class Unsupported(Exception):
    pass


def class_name(card_name):
    name = card_name.split(' // ')[0]
    name = re.sub(r"[^A-Za-z0-9 ]", "", name.replace('-', ' '))
    return ''.join(w[:1].upper() + w[1:] for w in name.split())


def subtype_names():
    names = {}
    for m in re.finditer(r'^\s*(\w+)\("([^"]+)"', open(SUBTYPES).read(), re.M):
        names[m.group(1)] = m.group(2)
    return names


def constructor_body(source, cls):
    # braces in a comment don't count (Foundry Assembler's "{1)")
    source = re.sub(r'(?m)^(\s*)//[^\n]*', r'\1', source)
    m = re.search(r'public\s+' + cls + r'\s*\(\s*UUID\s+(\w+)\s*,\s*CardSetInfo\s+(\w+)\s*\)\s*\{', source)
    if not m:
        raise Unsupported('no card constructor')
    owner, info = m.group(1), m.group(2)
    depth, i = 1, m.end()
    while depth:
        c = source[i]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
        i += 1
    body = source[m.end():i - 1]
    if (owner, info) != ('ownerId', 'setInfo'):
        body = re.sub(r'\b' + owner + r'\b', 'ownerId', re.sub(r'\b' + info + r'\b', 'setInfo', body))
    return body


def statements(body):
    body = re.sub(r'//[^\n]*', '', body)
    body = re.sub(r'/\*.*?\*/', '', body, flags=re.S)
    out, depth, start, quote = [], 0, 0, False
    for i, c in enumerate(body):
        if c == '"':
            quote = not quote
        elif quote:
            continue
        elif c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        elif c == ';' and depth == 0:
            out.append(canonical(body[start:i]))
            start = i + 1
    rest = canonical(body[start:])
    if rest:
        out.append(rest)
    return [s for s in out if s]


def canonical(text):
    """one statement on one line, with no spaces around punctuation: `this.power=new MageInt(2)`; string literals
    keep their text"""
    parts = re.split(r'("(?:[^"\\]|\\.)*")', text)
    out = []
    for i, part in enumerate(parts):
        if i % 2:
            out.append(part)
        else:
            part = ' '.join(part.split())
            out.append(re.sub(r'\s*([(),{}\[\]=])\s*', r'\1', part))
    return ''.join(out).strip()


def split_args(text):
    out, depth, start, quote = [], 0, 0, False
    for i, c in enumerate(text):
        if c == '"':
            quote = not quote
        elif quote:
            continue
        elif c in '({[':
            depth += 1
        elif c in ')}]':
            depth -= 1
        elif c == ',' and depth == 0:
            out.append(text[start:i])
            start = i + 1
    out.append(text[start:])
    return [a.strip() for a in out] if text.strip() else []


class Translator:
    """Reads one constructor. Local variables (`Effect effect = new X(…)`) hold what they were given."""

    def __init__(self, filters=None, conditions=None, subtypes=None, values=None):
        self.vars = {}
        # the card class's `DynamicValue` fields, by name
        self.values = values or {}
        # the card class's own filter and condition fields, by name
        self.filters = filters or {}
        self.conditions = conditions or {}
        self.subtypes = subtypes or {}

    def amount(self, arg):
        """An int, or a `DynamicValue` as the engine's `Value`"""
        arg = self.strip_setters(arg.strip())
        if re.fullmatch(r'-?\d+', arg):
            return int(arg)
        if (m := re.fullmatch(r'StaticValue\.get\((-?\d+)\)', arg)):
            return int(m.group(1))
        if (m := re.fullmatch(r'new GreatestAmongPermanentsValue\(Quality\.ManaValue,(.*)\)', arg)):
            return {'v': 'greatest_mana_value', 'filter': self.filter(m.group(1))}
        if arg in self.values:
            return self.amount(self.values[arg])
        # the life a "whenever you gain life" trigger saw is its X (The Ooze: the +1/+1 counters on what left)
        if arg in ('GetXValue.instance', 'ManacostVariableValue.REGULAR', 'ManacostVariableValue.ETB',
                   'SavedGainedLifeValue.MUCH', 'TheOozeValue.instance', 'SavedDamageValue.MUCH', 'SavedDamageValue.MANY',
                   'SavedLifeLossValue.MUCH'):
            return {'v': 'x'}
        if arg == 'UnlicensedHearseValue.instance':
            return {'v': 'exiled_with_source'}
        if arg == 'IntrepidAdversaryValue.instance':
            return {'v': 'source_counters', 'counter': 'valor'}
        if arg in ('SourcePermanentPowerValue.NOT_NEGATIVE', 'SourcePermanentPowerValue.ALLOW_NEGATIVE'):
            return {'v': 'source_power'}
        if arg == 'SourcePermanentToughnessValue.instance':
            return {'v': 'source_toughness'}
        if arg == 'ControllerLifeCount.instance':
            return {'v': 'controller_life'}
        if arg in COUNT_FILTERS:
            return {'v': 'permanents', 'filter': [dict(p) for p in COUNT_FILTERS[arg]]}
        if arg == 'CardTypesInGraveyardCount.ALL':
            return {'v': 'card_types_in_graveyards'}
        if arg == 'CardTypesInGraveyardCount.YOU':
            return {'v': 'card_types_in_your_graveyard'}
        if (m := re.fullmatch(r'DevotionCount\.([WUBRG])', arg)):
            return {'v': 'devotion', 'c': m.group(1)}
        if arg == 'DomainValue.REGULAR':
            return {'v': 'domain'}
        if arg == 'RevealTargetFromHandCostCount.instance':
            return {'v': 'x'}
        if arg == 'CardsDrawnThisTurnDynamicValue.instance':
            return {'v': 'cards_drawn'}
        if arg == 'KaitoBaneOfNightmaresCount.instance':
            return {'v': 'opponents_lost_life'}
        if arg == 'PermanentsSacrificedThisTurnCount.YOU':
            return {'v': 'sacrificed_this_turn'}
        if arg == 'CardsInControllerHandCount.ANY':
            return {'v': 'cards_in_hand'}
        obj = self.new_expr(arg)
        if isinstance(obj, tuple):
            cls, args = obj
            if cls == 'PermanentsOnBattlefieldCount' and len(args) in (1, 2):
                v = {'v': 'permanents', 'filter': self.filter(args[0])}
                if len(args) == 2 and args[1].strip() != 'null':
                    # the second argument multiplies
                    v = {'v': 'times', 'of': v, 'n': int(args[1])}
                return v
            # +N/+N for each Aura attached to it
            if cls == 'AuraAttachedCount' and len(args) <= 1:
                v = {'v': 'auras_attached'}
                return {'v': 'times', 'of': v, 'n': int(args[0])} if args else v
            if cls == 'CardsInControllerGraveyardCount' and len(args) <= 1:
                return {'v': 'cards_in_graveyard', 'filter': self.filter(args[0]) if args else []}
            if cls == 'IntPlusDynamicValue' and len(args) == 2 and re.fullmatch(r'-?\d+', args[0].strip()):
                return {'v': 'plus', 'of': self.amount(args[1]), 'n': int(args[0])}
            if cls == 'SignInversionDynamicValue' and len(args) == 1:
                return {'v': 'negative', 'of': self.amount(args[0])}
            if cls == 'MultipliedValue' and len(args) == 2 and re.fullmatch(r'-?\d+', args[1].strip()):
                return {'v': 'times', 'of': self.amount(args[0]), 'n': int(args[1])}
            if cls == 'HalfValue' and len(args) == 2 and args[1].strip() == 'false':
                return {'v': 'half', 'of': self.amount(args[0])}
        raise Unsupported('amount ' + arg)

    def condition(self, arg):
        """a condition argument: a known instance, a field of the card, or `new XCondition(…)`"""
        arg = canonical(arg)
        if arg in CONDITIONS:
            return CONDITIONS[arg]
        if arg in self.conditions:
            return self.condition(self.conditions[arg])
        if arg == 'SourceEnteredOrControlsBasicLandCondition.instance':
            return {'c': 'or', 'xs': [{'c': 'source_entered_this_turn'}, self.condition('YouControlABasicLandCondition.instance')]}
        if arg == 'YouControlABasicLandCondition.instance':
            return {'c': 'permanents', 'filter': [LAND, {'p': 'supertype', 's': 'BASIC'}], 'cmp': 'OR_GREATER', 'n': 1}
        if (m := re.fullmatch(r'new (YouControlPermanentCondition|PermanentsOnTheBattlefieldCondition)\((.*)\)', arg)):
            args = split_args(m.group(2))
            out = {'c': 'permanents', 'filter': self.filter(args[0]), 'cmp': 'OR_GREATER', 'n': 1}
            if len(args) >= 3 and (c := re.fullmatch(r'ComparisonType\.(\w+)', args[1])) and c.group(1) in COMPARISONS \
                    and re.fullmatch(r'\d+', args[2]):
                out['cmp'], out['n'] = c.group(1), int(args[2])
            elif not (len(args) == 1 or (len(args) == 2 and args[1] in ('true', 'false'))):
                raise Unsupported('condition ' + arg)
            # PermanentsOnTheBattlefieldCondition counts only yours unless told otherwise (`onlyControlled`)
            if m.group(1) == 'PermanentsOnTheBattlefieldCondition' and args[-1] == 'false' and len(args) in (2, 4):
                out['all'] = True
            return out
        if (m := re.fullmatch(r'new LifeCompareCondition\(TargetController\.(YOU|OPPONENT),ComparisonType\.(\w+),(\d+)\)', arg)) \
                and m.group(2) in COMPARISONS:
            return {'c': 'life_compare', 'who': m.group(1).lower(), 'cmp': m.group(2), 'n': int(m.group(3))}
        if (m := re.fullmatch(r'new CardsInControllerGraveyardCondition\((\d+)(?:,(.*))?\)', arg)):
            return {'c': 'cards_in_graveyard', 'n': int(m.group(1)), 'filter': self.filter(m.group(2)) if m.group(2) else []}
        if (m := re.fullmatch(r'new SourceHasCounterCondition\((CounterType\.\w+),ComparisonType\.OR_GREATER,(\d+)\)', arg)) \
                and m.group(1) in COUNTERS:
            return {'c': 'source_counter', 'name': COUNTERS[m.group(1)], 'n': int(m.group(2))}
        if (m := re.fullmatch(r'new SourceHasCounterCondition\((CounterType\.\w+)(?:,(\d+))?\)', arg)) and m.group(1) in COUNTERS:
            return {'c': 'source_counter', 'name': COUNTERS[m.group(1)], 'n': int(m.group(2) or 1)}
        if (m := re.fullmatch(r'new TargetHasCardTypeCondition\(CardType\.([A-Z]+)\)', arg)):
            return {'c': 'source_targets', 'filter': [{'p': 'type', 't': m.group(1)}]}
        if (m := re.fullmatch(r'new SourceTargetsPermanentCondition\((.*)\)', arg)):
            return {'c': 'source_targets', 'filter': self.filter(m.group(1))}
        if (m := re.fullmatch(r'new OpponentControlsMoreCondition\((.*)\)', arg)):
            return {'c': 'opponent_controls_more', 'filter': self.filter(m.group(1))}
        if (m := re.fullmatch(r'VoiceOfTheBlessedCondition\.(FOUR|TEN)', arg)):
            return {'c': 'source_counter', 'name': '+1/+1', 'n': 4 if m.group(1) == 'FOUR' else 10}
        if arg == 'ArclightPhoenixCondition.instance':
            return {'c': 'instants_sorceries_cast', 'n': 3}
        if arg == 'MysticalDisputeCondition.instance':
            return {'c': 'source_targets', 'filter': [{'p': 'color', 'c': 'U'}]}
        if (m := re.fullmatch(r'new CompoundCondition\((.*)\)', arg)):
            args = split_args(m.group(1))
            if args and args[0].strip().startswith('"'):
                args = args[1:]
            return {'c': 'and', 'xs': [self.condition(a) for a in args]}
        if (m := re.fullmatch(r'new InvertCondition\((.*)\)', arg)):
            args = split_args(m.group(1))
            if len(args) == 2 and args[1].strip().startswith('"'):
                args = args[:1]
            if len(args) == 1:
                return {'c': 'not', 'x': self.condition(args[0])}
        # an anonymous subclass that only overrides its text
        if (m := re.fullmatch(r'(new \w+Condition\(.*?\))\s*\{\s*@Override\s*public String toString\(\)\s*\{\s*return\s*"[^"]*"[;}\s]*', arg, re.S)):
            return self.condition(m.group(1))
        if (m := re.fullmatch(r'new CardsInExileCondition\(ComparisonType\.OR_GREATER,(\d+)\)', arg)):
            return {'c': 'cards_in_exile', 'n': int(m.group(1))}
        if (m := re.fullmatch(r'new YouGainedLifeCondition\(ComparisonType\.(\w+),(\d+)\)', arg)) and m.group(1) in COMPARISONS:
            return {'c': 'life_gained', 'cmp': m.group(1), 'n': int(m.group(2))}
        # the starting life total is 20 in every game the engine plays
        if (m := re.fullmatch(r'MoreThanStartingLifeTotalCondition\.(ONE|SEVEN|TEN|FIFTEEN)', arg)):
            n = {'ONE': 1, 'SEVEN': 7, 'TEN': 10, 'FIFTEEN': 15}[m.group(1)]
            return {'c': 'life_compare', 'who': 'you', 'cmp': 'OR_GREATER', 'n': 20 + n}
        raise Unsupported('condition ' + arg)

    def filter(self, arg):
        """a filter argument: a field of the card, a `StaticFilters` one, or `new FilterX(…)`"""
        arg = arg.strip()
        name = arg[len('StaticFilters.'):] if arg.startswith('StaticFilters.') else arg
        found = self.filters.get(name)
        if found is None and re.fullmatch(r'FILTER_\w+', name):
            found = static_filters(self.subtypes).get(name)
        if found is None and (m := re.fullmatch(r'new (Filter\w+)\((.*)\)', arg, re.S)):
            found = filter_object(m.group(1), split_args(m.group(2)), [], self.subtypes)
        if isinstance(found, Unsupported):
            raise found
        if found is None:
            raise Unsupported('filter ' + arg)
        return found

    @staticmethod
    def strip_setters(expr):
        """chained setters that only change texts or hints: `….setText("…")`, `….addHint(x.getHint())`"""
        expr = expr.strip()
        while expr.endswith(')'):
            depth, i = 0, len(expr) - 1
            while i >= 0:
                depth += {')': 1, '(': -1}.get(expr[i], 0)
                if depth == 0:
                    break
                i -= 1
            m = re.search(r'\.(\w+)$', expr[:i])
            if not m or m.group(1) not in TEXT_SETTERS:
                break
            expr = expr[:m.start()].strip()
        return expr

    def new_expr(self, expr):
        """`new X(args)`, `X.getInstance()` or a local variable -> (X, [args]) or the variable's object"""
        expr = self.strip_setters(expr)
        if expr in self.vars:
            return self.vars[expr]
        m = re.fullmatch(r'new (\w+)\(\)', expr)
        if m and m.group(1) in CUSTOM and CUSTOM[m.group(1)]:
            return triggered(CUSTOM[m.group(1)]())
        # whenever a permanent of the filter you control enters
        m = re.fullmatch(r'new EntersBattlefieldControlledTriggeredAbility\((.*)\)', expr, re.S)
        if m:
            args = split_args(m.group(1))
            if args[0].strip() == 'Zone.BATTLEFIELD':
                args = args[1:]
            pointer = False
            if args and args[-1].strip().startswith('SetTargetPointer.'):
                if args[-1].strip() not in ('SetTargetPointer.PERMANENT', 'SetTargetPointer.NONE'):
                    raise Unsupported('EntersBattlefieldControlledTriggeredAbility(' + m.group(1) + ')')
                pointer = args[-1].strip() == 'SetTargetPointer.PERMANENT'
                args = args[:-1]
            if len(args) not in (2, 3) or (len(args) == 3 and args[2].strip() not in ('true', 'false')):
                raise Unsupported('EntersBattlefieldControlledTriggeredAbility(' + m.group(1) + ')')
            return triggered({'class': 'EntersBattlefieldAllTriggeredAbility',
                              'java': 'EntersBattlefieldControlledTriggeredAbility',
                              'effects': [self.effect(args[0])], 'filter': self.filter(args[1]) + [YOU],
                              'optional': len(args) == 3 and args[2].strip() == 'true',
                              'set_target_pointer': pointer})
        m = re.fullmatch(r'(\w+)\.getInstance\(\)', expr)
        if m:
            return m.group(1), []
        m = re.fullmatch(r'new (\w+)(?:<>)?\((.*)\)', expr)
        if not m:
            raise Unsupported('expression ' + expr)
        return m.group(1), split_args(m.group(2))

    def value(self, kind, arg):
        if kind == 'int':
            if re.fullmatch(r'-?\d+', arg):
                return int(arg)
        elif kind == 'amount':
            return self.amount(arg)
        elif kind == 'bool':
            if arg in ('true', 'false'):
                return arg == 'true'
        elif kind == 'false':
            if arg == 'false':
                return False
        elif kind == 'no_pointer':
            if arg == 'SetTargetPointer.NONE':
                return None
        elif kind == 'duration':
            if arg in DURATIONS:
                return DURATIONS[arg]
        elif kind == 'filter':
            return self.filter(arg)
        elif kind == 'string':
            # a literal, or a constant of the card's (`rule`)
            if re.fullmatch(r'"[^"]*"(?:\s*\+\s*"[^"]*")*', arg) or arg == 'null' or re.fullmatch(r'[a-z]\w*', arg):
                return None
        elif kind == 'effect':
            return None if arg == 'null' else self.effect(arg)
        elif kind == 'ability':
            return self.ability(arg)
        elif kind == 'count':
            if arg in COUNTS:
                return COUNTS[arg]
        elif kind == 'life_cost':
            m = re.fullmatch(r'new PayLifeCost\((\d+)\)', arg)
            if m:
                return int(m.group(1))
        elif kind == 'target_controller':
            if arg in TARGET_CONTROLLERS:
                return TARGET_CONTROLLERS[arg]
        elif kind == 'condition':
            return self.condition(arg)
        elif kind == 'zone':
            if arg in ('Zone.BATTLEFIELD',):
                return None
        elif kind == 'counter_type':
            if arg in COUNTERS:
                return COUNTERS[arg]
        elif kind == 'outcome':
            if re.fullmatch(r'Outcome\.\w+', arg):
                return None
        elif kind == 'aura':
            if arg in ('AttachmentType.AURA', 'AttachmentType.EQUIPMENT'):
                return None
        elif kind == 'while_on_battlefield':
            if arg == 'Duration.WhileOnBattlefield':
                return None
        elif kind == 'subtype':
            if re.fullmatch(r'SubType\.\w+', arg):
                return subtype_names()[arg[8:]]
        elif kind == 'this':
            if arg == 'this':
                return None
        elif kind == 'mana_costs':
            m = re.fullmatch(r'new ManaCostsImpl<>\("(\{[^"]*\})"\)', arg)
            if m:
                return m.group(1)
        elif kind == 'cost_text':
            if re.fullmatch(r'"(\{[^"]*\})"', arg):
                return arg[1:-1]
        elif kind == 'cost':
            return [self.cost(arg)]
        elif kind == 'counter':
            m = re.fullmatch(r'(CounterType\.\w+)\.createInstance\((\d*)\)', arg)
            if m and m.group(1) in COUNTERS:
                return {'name': COUNTERS[m.group(1)], 'amount': int(m.group(2) or 1)}
        elif kind == 'creature_token':
            return self.creature_token(arg)
        elif kind == 'cast_adjustment':
            if arg in ('CastManaAdjustment.NONE', 'CastManaAdjustment.AS_THOUGH_ANY_MANA_TYPE',
                       'CastManaAdjustment.AS_THOUGH_ANY_MANA_COLOR'):
                return arg != 'CastManaAdjustment.NONE'
        elif kind == 'token':
            m = re.fullmatch(r'new (\w+Token)\(\)', arg)
            if m:
                return token(m.group(1))
        raise Unsupported('argument ' + arg)

    def cost(self, expr):
        if isinstance(self.vars.get(expr.strip()), dict) and self.vars[expr.strip()].get('class', '').endswith('Cost'):
            return self.vars[expr.strip()]
        # "exile N cards from your graveyard" (Emeritus of Ideation; not with the source's exile zone)
        if (m := re.fullmatch(r'new ExileFromGraveCost\(new TargetCardInYourGraveyard\((\d+)\)\)(?:\.withSourceExileZone\(false\))?',
                              canonical(expr))):
            return {'class': 'ExileFromGraveCost', 'amount': int(m.group(1))}
        cls, args = self.new_expr(expr)
        if cls == 'ManaCostsImpl' and len(args) == 1 and re.fullmatch(r'"[^"]*"', args[0]):
            return {'class': 'ManaCost', 'cost': args[0][1:-1]}
        if cls == 'ColoredManaCost' and len(args) == 1 and (n := re.fullmatch(r'ColoredManaSymbol\.([WUBRG])', args[0].strip())):
            return {'class': 'ManaCost', 'cost': '{' + n.group(1) + '}'}
        if cls == 'GenericManaCost' and len(args) == 1 and re.fullmatch(r'\d+', args[0]):
            return {'class': 'ManaCost', 'cost': '{' + args[0] + '}'}
        if cls in ('TapSourceCost', 'SacrificeSourceCost', 'ExileSourceCost', 'ExileSourceFromGraveCost', 'ForageCost', 'ExertSourceCost') and not args:
            return {'class': cls}
        if cls == 'RemoveCounterCost' and len(args) == 3 and args[1].strip() in COUNTERS and args[2].strip().isdigit() \
                and (n := re.fullmatch(r'new TargetControlledCreaturePermanent\(1,(\d+)\)', canonical(args[0]))):
            return {'class': 'RemoveCountersFromAmongCost', 'filter': [CREATURE], 'counter': COUNTERS[args[1].strip()],
                    'amount': int(args[2]), 'max': int(n.group(1))}
        if cls == 'PutCountersSourceCost' and len(args) == 1:
            return {'class': cls, 'counter': self.value('counter', args[0].strip())}
        if cls == 'RemoveCountersSourceCost' and len(args) == 1:
            return {'class': cls, 'counter': self.value('counter', args[0].strip())}
        if cls == 'OrCost' and len(args) == 3 and re.fullmatch(r'"[^"]*"', args[0].strip()):
            return {'class': cls, 'costs': [self.cost(args[1]), self.cost(args[2])]}
        if cls == 'TapTargetCost' and len(args) == 1:
            # (n, n, filter, notTarget): exactly n
            n = re.fullmatch(r'new TargetControlledPermanent\((\d+),(\d+),(.*),true\)', canonical(args[0]))
            if n and n.group(1) == n.group(2):
                return {'class': cls, 'filter': self.filter(n.group(3)), 'amount': int(n.group(1))}
            n = re.fullmatch(r'new TargetControlledPermanent\((?:(\d+),)?(.*)\)', args[0].strip())
            if n:
                return {'class': cls, 'filter': self.filter(n.group(2)), 'amount': int(n.group(1) or 1)}
            return {'class': cls, 'filter': self.filter(args[0]), 'amount': 1}
        if cls == 'TapTargetCost' and len(args) == 2 and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'filter': self.filter(args[1]), 'amount': int(args[0])}
        # "As an additional cost to cast this spell, you may sacrifice any number of …"
        # Merchant's Dockhand's own cost class: tap X untapped artifacts you control
        if cls == 'TapXTargetCost' and not args:
            return {'class': 'TapXTargetCost', 'filter': [type_pred('ARTIFACT')]}
        if cls == 'SacrificeXTargetCost' and len(args) == 2 and args[1].strip() == 'true':
            return {'class': cls, 'filter': self.filter(args[0])}
        if cls == 'SacrificeTargetCost' and len(args) == 1:
            return {'class': cls, 'filter': self.filter(args[0])}
        if cls == 'SacrificeTargetCost' and len(args) == 2 and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'filter': self.filter(args[1]), 'amount': int(args[0])}
        if cls in ('RevealTargetFromHandCost', 'ExileFromHandCost') and len(args) == 1:
            n = re.fullmatch(r'new TargetCardInHand\((?:(\d+),(\d+|Integer\.MAX_VALUE),)?(.*)\)', canonical(args[0]))
            if n:
                out = {'class': cls, 'filter': self.filter(n.group(3))}
                lo, hi = (n.group(1), n.group(2)) if n.group(1) else ('1', '1')
                if cls == 'RevealTargetFromHandCost':
                    out['min'], out['max'] = int(lo), 99 if hi == 'Integer.MAX_VALUE' else int(hi)
                elif (lo, hi) != ('1', '1'):
                    raise Unsupported('cost ' + cls)
                return out
        # "discard two cards"
        if cls == 'DiscardTargetCost' and len(args) == 1 \
                and (n := re.fullmatch(r'new TargetCardInHand\((\d+),(.*)\)', canonical(args[0]))):
            return {'class': 'DiscardCardCost', 'filter': self.filter(n.group(2)), 'amount': int(n.group(1))}
        if cls == 'DiscardCardCost' and len(args) <= 1 and args != ['true']:
            return {'class': cls, 'filter': self.filter(args[0]) if args and args[0] != 'false' else []}
        if cls == 'BeholdCost' and len(args) == 1 and (m := re.fullmatch(r'SubType\.(\w+)', args[0].strip())):
            return {'class': cls, 'subtype': subtype_names()[m.group(1)]}
        if cls == 'PayEnergyCost' and len(args) == 1 and re.fullmatch(r'\d+', args[0]):
            return {'class': cls, 'amount': int(args[0])}
        if cls == 'PayLifeCost' and len(args) == 1 and re.fullmatch(r'\d+', args[0]):
            return {'class': cls, 'amount': int(args[0])}
        raise Unsupported('cost ' + expr)

    def build(self, cls, args, spec, what):
        if len(args) > len(spec) or any(k not in ('bool', 'string', 'false', 'no_pointer', 'outcome', 'while_on_battlefield') and n not in DEFAULTS
                                        for n, k in spec[len(args):]):
            raise Unsupported(what + ' ' + cls + '(' + ','.join(args) + ')')
        out = {'class': cls}
        for name, _ in spec[len(args):]:
            if name in DEFAULTS:
                out[name] = DEFAULTS[name]
        for (name, kind), arg in zip(spec, args):
            # the engine's effect numbers may be dynamic
            if what == 'effect' and kind == 'int' and name in ('amount', 'power', 'toughness'):
                kind = 'amount'
            v = self.value(kind, arg)
            if name:
                out[name] = v
        return out

    def effect(self, expr):
        # untap every target's permanents (Nissa, Genesis Mage's creatures and lands)
        if re.fullmatch(r'new UntapTargetEffect\((?:"[^"]*")?\)\.setTargetPointer\(new EachTargetPointer\(\)\)', canonical(expr.strip())):
            return {'class': 'UntapEachTargetEffect'}
        # every target (the one target a duel's "for each opponent" adjuster leaves)
        expr = re.sub(r'\.setTargetPointer\(new EachTargetPointer\(\)\)((?:\.setText\(.*\))?)$', r'\1', expr.strip(), flags=re.S)
        # the ability's second target (`SecondTargetPointer`): counters put on it
        m = re.fullmatch(r'(new AddCountersTargetEffect\(.*\))\.setTargetPointer\(new SecondTargetPointer\(\)\)', canonical(self.strip_setters(expr)), re.S)
        if m:
            e = self.effect(m.group(1))
            if e.get('class') == 'AddCountersTargetEffect':
                e['second'] = True
                return e
        m = re.fullmatch(r'new RemoveCounterTargetEffect\((CounterType\.\w+)\.createInstance\(\)\)', canonical(self.strip_setters(expr)))
        if m and m.group(1) in COUNTERS:
            return {'class': 'RemoveCounterTargetEffect', 'counter': COUNTERS[m.group(1)]}
        # `BoostControlledEffect(p, t, duration, excludeSource)`: creatures you control
        m = re.fullmatch(r'new BoostControlledEffect\((-?\d+),(-?\d+),(Duration\.\w+),(true|false)\)', canonical(self.strip_setters(expr)))
        if m:
            return self.effect('new BoostControlledEffect({},{},{},StaticFilters.FILTER_PERMANENT_CREATURES,{})'.format(*m.groups()))
        # exiled "with" the source (Ghost Vacuum)
        if self.strip_setters(expr) == 'new ExileTargetEffect().setToSourceExileZone(true)':
            return {'class': 'ExileTargetForSourceEffect'}
        # a cost reduction that only counts while the spell is cast
        if (m := re.fullmatch(r'(.*)\.setCanWorksOnStackOnly\(true\)', expr.strip(), re.S)):
            e = self.effect(m.group(1))
            if e.get('class') != 'SpellCostReductionSourceEffect':
                raise Unsupported('setCanWorksOnStackOnly on ' + e.get('class'))
            e['stack_only'] = True
            return e
        m = re.fullmatch(r'(new CreateTokenEffect\(.*\))\.withAdditionalTokens\((new \w+Token\(\))\)', self.strip_setters(expr), re.S)
        if m and balanced(m.group(1)):
            e = self.effect(m.group(1))
            e['also'] = self.value('token', m.group(2))
            return e
        # `new DoIfCostPaid(…).addEffect(…)`: more effects if it's paid
        m = re.fullmatch(r'(new DoIfCostPaid\(.*\))\.addEffect\((new .*)\)', self.strip_setters(expr), re.S)
        if m and balanced(m.group(1)):
            e = self.effect(m.group(1))
            e.setdefault('also', []).append(self.effect(m.group(2)))
            return e
        # attacks each combat if able (while a condition holds)
        m = re.fullmatch(r'new ConditionalRequirementEffect\(new AttacksIfAbleSourceEffect\(Duration\.WhileOnBattlefield(?:,(?:true|false))?\),(.*?)(?:,"[^"]*")?\)', canonical(self.strip_setters(expr)), re.S)
        if m:
            return {'class': 'AttacksIfAbleSourceEffect', 'condition': self.condition(m.group(1))}
        if re.fullmatch(r'new AttacksIfAbleSourceEffect\(Duration\.WhileOnBattlefield(?:,(?:true|false))?\)', canonical(self.strip_setters(expr))):
            return {'class': 'AttacksIfAbleSourceEffect'}
        if expr.strip() == 'PlayFromGraveyardControllerEffect.playLands()':
            return {'class': 'PlayLandsFromGraveyardControllerEffect'}
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            return obj
        cls, args = obj
        # `new ConditionalOneShotEffect(…).addEffect(e)`: e too, when it holds
        m = re.fullmatch(r'(new ConditionalOneShotEffect\(.*\))\s*\.addEffect\((.*)\)', expr.strip(), re.S)
        if m and balanced(m.group(1)) and balanced(m.group(2)):
            e = self.effect(m.group(1))
            e.setdefault('also', []).append(self.effect(m.group(2)))
            return e
        # "if a permanent dealt damage by {this} would die this turn, exile it instead": a single-target damage spell's
        # target
        if cls == 'DealtDamageToCreatureBySourceDies' and [a.strip() for a in args] == ['this', 'Duration.EndOfTurn']:
            return {'class': 'ExileTargetIfDiesEffect'}
        # `new DoWhenCostPaid(reflexive, cost, "Forage?")`
        if cls == 'DoWhenCostPaid' and len(args) == 3 and re.fullmatch(r'"[^"]*"', args[2].strip()):
            r = self.vars.get(args[0].strip()) or self.ability(args[0])
            if r.get('class') != 'Reflexive' or [e['class'] for e in r['effects']] != ['ReturnToHandTargetEffect']:
                raise Unsupported('reflexive ability ' + args[0])
            return {'class': cls, 'cost': [self.cost(args[1])], 'message': args[2].strip()[1:-1],
                    'rule': 'return target permanent card from your graveyard to your hand.',
                    'targets': r['targets'], 'effects': r['effects']}
        # `new DoIfCostPaid(null, otherwise, cost)`: only what's done when it isn't paid
        # `new DoIfCostPaid(effect, null, cost, optional)`
        if cls == 'DoIfCostPaid' and len(args) == 4 and args[1].strip() == 'null' and args[3].strip() in ('true', 'false'):
            return {'class': cls, 'effect': self.effect(args[0]), 'cost': [self.cost(args[2])],
                    'optional': args[3].strip() == 'true'}
        if cls == 'DoIfCostPaid' and len(args) == 3 and args[0].strip() == 'null':
            return {'class': cls, 'cost': [self.cost(args[2])], 'otherwise': [self.effect(args[1])]}
        # Skyclave Apparition: an X/X token for the cards its source exiled
        if cls == 'CreateXXTokenExiledEffectManaValueEffect' and len(args) == 2 \
                and args[0].strip() == 'CustomIllusionToken::new':
            illusion = {'class': 'CustomIllusionToken', 'name': 'Illusion Token', 'types': ['CREATURE'], 'colors': ['U'], 'subtypes': ['Illusion'],
                        'abilities': []}
            if EXTRA.get('token:CustomIllusionToken', {}).get('images'):
                illusion['images'] = EXTRA['token:CustomIllusionToken']['images']
            return {'class': 'CreateXXTokenExiledEffect', 'token': illusion}
        # a one-shot wrapper that starts a continuous effect: the effect itself
        if cls == 'AddContinuousEffectToGame' and len(args) == 1:
            return self.effect(args[0])
        if cls == 'DamageTargetEffect' and args == ['SourcePermanentPowerValue.NOT_NEGATIVE']:
            return {'class': 'DamageTargetSourcePowerEffect'}
        if cls.startswith('SearchLibraryPutIn'):
            return self.search_library(cls, [a.strip() for a in args])
        if cls == 'PlayAdditionalLandsControllerEffect' and len(args) == 2 and args[1].strip() == 'Duration.WhileOnBattlefield' \
                and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'amount': int(args[0])}
        # Urborg: "Each land is a Swamp in addition to its other land types"
        if cls == 'AddBasicLandTypeAllLandsEffect' and len(args) == 1 \
                and args[0].strip() in ('SubType.PLAINS', 'SubType.ISLAND', 'SubType.SWAMP', 'SubType.MOUNTAIN', 'SubType.FOREST'):
            return {'class': cls, 'subtype': subtype_names()[args[0].strip()[8:]]}
        if cls == 'RottenmouthViperEffect' and [a.strip() for a in args] == ['new CountersSourceCount(CounterType.BLIGHT)']:
            return {'class': cls}
        # Laughing Jasper Flint: "creatures you control but don't own are Mercenaries in addition to their other types"
        if cls == 'AddCardSubtypeAllEffect' and len(args) == 3 and re.fullmatch(r'SubType\.\w+', args[1].strip()) \
                and args[2].strip() == 'DependencyType.AddingCreatureType':
            return {'class': cls, 'filter': self.filter(args[0].strip()), 'subtype': subtype_names()[args[1].strip()[8:]]}
        # Evolved Sleeper: "becomes a Human Cleric with base power and toughness 2/2" until it leaves
        if cls == 'AddCardSubTypeSourceEffect' and len(args) > 1 and args[0].strip() == 'Duration.Custom' \
                and all(re.fullmatch(r'SubType\.\w+', a.strip()) for a in args[1:]):
            names = subtype_names()
            return {'class': 'BecomesSubtypesSourceEffect', 'subtypes': [names[a.strip()[8:]] for a in args[1:]]}
        if cls == 'SetBasePowerToughnessSourceEffect' and len(args) == 3 and args[2].strip() == 'Duration.Custom' \
                and all(re.fullmatch(r'\d+', a.strip()) for a in args[:2]):
            return {'class': 'SetBasePowerToughnessUntilLeavesEffect', 'power': int(args[0]), 'toughness': int(args[1])}
        if cls == 'EarthbendTargetEffect' and len(args) == 1:
            return {'class': cls, 'amount': self.value('amount', args[0].strip())}
        if cls == 'EmpowerJaceEffect' and len(args) == 1 and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'amount': self.amount(args[0].strip()), 'token': token('JacePlaneswalkerToken')}
        # a Role token attached to each target
        if cls == 'CreateRoleAttachedTargetEffect' and len(args) == 1 and (m := re.fullmatch(r'RoleType\.(\w+)', args[0].strip())):
            return {'class': cls, 'token': token(m.group(1).title().replace('_', '') + 'RoleToken')}
        if cls == 'LoseAllAbilitiesTargetEffect' and [a.strip() for a in args] == ['Duration.UntilSourceLeavesBattlefield']:
            return {'class': 'LoseAllAbilitiesUntilSourceLeavesEffect'}
        if cls == 'AddManaOfAnyColorEffect' and len(args) == 1 and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'amount': int(args[0])}
        if cls == 'AddManaOfAnyColorEffect' and not args:
            return {'class': cls, 'amount': 1}
        # "target creature blocks this turn if able": a requirement it has until end of turn
        if cls == 'BlocksIfAbleTargetEffect' and [a.strip() for a in args] == ['Duration.EndOfTurn']:
            return {'class': 'GainAbilityTargetEffect', 'ability': {'class': 'BlocksIfAbleAbility'}, 'duration': 'EndOfTurn'}
        # Urza's Saga's chapters: it gains the ability for as long as it's on the battlefield
        if cls == 'GainAbilitySourceEffect' and len(args) == 2 and args[1].strip() == 'Duration.Custom':
            gained = self.ability(args[0])
            if gained.get('class') == 'SimpleActivatedAbility':
                # (Urza's Saga's: its `getRule`)
                gained.setdefault('rule', '{2}, {T}: Create a 0/0 colorless Construct artifact creature token with '
                                          '"This token gets +1/+1 for each artifact you control."'
                                  if gained.get('effects', [{}])[0].get('token', {}).get('class') == 'KarnConstructToken' else '')
            return {'class': 'GainAbilityPermanentlyEffect', 'ability': gained}
        # "this can't be blocked this turn"
        if cls == 'CantBeBlockedSourceEffect' and [a.strip() for a in args] == ['Duration.EndOfTurn']:
            return {'class': 'GainAbilitySourceEffect', 'ability': {'class': 'CantBeBlockedSourceAbility'}, 'duration': 'EndOfTurn'}
        # Ephemerate: exile, then return under its owner's control
        if cls == 'ExileThenReturnTargetEffect' and [a.strip() for a in args] in (['false', 'false'], ['false', 'false', 'PutCards.BATTLEFIELD']):
            return {'class': cls}
        # its rule text only
        if cls == 'ExileTargetEffect' and len(args) == 1 and re.fullmatch(r'"[^"]*"', args[0].strip()):
            return {'class': cls}
        # Bloodthirsty Adversary: pay {2}{R} any number of times; when you do, that many counters and copies
        if cls == 'DoIfAnyNumberCostPaid' and len(args) == 2 and args[0].strip() == 'new BloodthirstyAdversaryEffect()' \
                and (m := re.fullmatch(r'new ManaCostsImpl<>\("(\{[^"]*\})"\)', args[1].strip())):
            return {'class': 'BloodthirstyAdversaryEffect', 'cost': m.group(1)}
        if cls == 'ChooseACardNameEffect' and [a.strip() for a in args] in (['ChooseACardNameEffect.TypeOfName.LAND_NAME'], ['ChooseACardNameEffect.TypeOfName.ALL']):
            return {'class': 'ChooseLandNameEffect'}
        if cls == 'ChooseCreatureTypeEffect' and len(args) == 1 and args[0].strip().startswith('Outcome.'):
            return {'class': cls}
        if cls == 'ChooseBasicLandTypeEffect' and len(args) == 1 and args[0].strip().startswith('Outcome.'):
            return {'class': cls}
        if cls in ('CreateTokenCopyTargetEffect', 'CreateTokenCopySourceEffect') and not args:
            return {'class': cls}
        if cls == 'TapSourceUnlessPaysEffect' and len(args) == 1 and (
                m := re.fullmatch(r'new BeholdCost\(SubType\.(\w+)\)', args[0].strip())):
            return {'class': 'TapSourceUnlessBeholdEffect', 'subtype': subtype_names()[m.group(1)]}
        if cls == 'TransformSourceEffect' and not args:
            return {'class': cls}
        if cls == 'CreateTwiceThatManyTokensEffect' and not args:
            return {'class': cls}
        if cls == 'MaximumHandSizeControllerEffect' and [a.strip() for a in args] == [
                'Integer.MAX_VALUE', 'Duration.WhileOnBattlefield', 'MaximumHandSizeControllerEffect.HandSizeModification.SET']:
            return {'class': 'NoMaximumHandSizeEffect'}
        if cls == 'AddCountersTargetEffect' and len(args) == 2:
            return {'class': cls, 'counter': self.value('counter', args[0].strip()), 'value': self.amount(args[1])}
        # (effect, condition[, text])
        if cls == 'ConditionalOneShotEffect' and len(args) in (2, 3) \
                and (re.fullmatch(r'\w+Condition\.\w+', args[1].strip()) or args[1].strip() in self.conditions
                     or re.fullmatch(r'new InvertCondition\(\w+Condition\.\w+\)', args[1].strip())
                     or re.fullmatch(r'new \w+Condition\(.*\)', args[1].strip(), re.S)):
            return {'class': cls, 'effect': self.effect(args[0]), 'condition': self.condition(args[1])}
        # (effect, otherwise, condition, text)
        if cls == 'ConditionalOneShotEffect' and len(args) == 4:
            return {'class': cls, 'effect': self.effect(args[0]), 'otherwise': self.effect(args[1]),
                    'condition': self.condition(args[2])}
        # a spell's "+2/+2, or +4/+4 instead if …": worked out as it resolves (`LockedInCondition`)
        if cls == 'ConditionalContinuousEffect' and len(args) == 4 \
                and all(re.fullmatch(r'new Boost(Target|Source)Effect\(-?\d+,-?\d+,Duration\.EndOfTurn\)', a.strip()) for a in args[:2]):
            c = args[2].strip()
            if (l := re.fullmatch(r'new LockedInCondition\((.*)\)', c, re.S)):
                c = l.group(1)
            return {'class': 'ConditionalOneShotEffect', 'effect': self.effect(args[0]), 'otherwise': self.effect(args[1]),
                    'condition': self.condition(c)}
        if cls == 'DrawDiscardTargetEffect' and len(args) == 2 and all(re.fullmatch(r'\d+', a.strip()) for a in args):
            return {'class': cls, 'draw': int(args[0]), 'discard': int(args[1])}
        if cls == 'CreateTokenTargetEffect' and len(args) == 1 and (m := re.fullmatch(r'new (\w+Token)\(\)', args[0].strip())):
            return {'class': cls, 'token': token(m.group(1))}
        if cls == 'CreateTokenTargetEffect' and len(args) == 2 and (m := re.fullmatch(r'new (\w+Token)\(\)', args[0].strip())):
            return {'class': cls, 'token': token(m.group(1)), 'amount': self.value('amount', args[1])}
        # "it becomes an artifact creature until end of turn"
        if cls == 'AddCardTypeSourceEffect' and len(args) >= 2 and args[0].strip() == 'Duration.EndOfTurn' \
                and all(re.fullmatch(r'CardType\.[A-Z]+', a.strip()) for a in args[1:]):
            return {'class': cls, 'types': [a.strip()[9:] for a in args[1:]], 'duration': 'EndOfTurn'}
        if cls == 'GainAbilityTargetEffect' and len(args) == 3 and re.fullmatch(r'"[^"]*"', args[2].strip()):
            return self.effect('new GainAbilityTargetEffect(' + args[0] + ',' + args[1] + ')')
        if cls in ('UntapAllControllerEffect', 'UntapAllEffect') and 1 <= len(args) <= 2:
            return {'class': 'UntapAllEffect', 'filter': self.filter(args[0])}
        if cls == 'DamageWithPowerFromOneToAnotherTargetEffect' and len(args) == 1 and re.fullmatch(r'"[^"]*"', args[0].strip()):
            return {'class': cls}
        if cls == 'DamageAllControlledTargetEffect' and 1 <= len(args) <= 2:
            return {'class': cls, 'amount': self.amount(args[0]),
                    'filter': self.filter(args[1]) if len(args) == 2 else [CREATURE]}
        if cls == 'DistributeCountersEffect' and [a.strip() for a in args] == ['CounterType.M1M1']:
            return {'class': 'DistributeM1M1CountersEffect'}
        if cls == 'PreventDamageByChosenSourceEffect' and len(args) == 2 and args[1].strip() == 'true':
            return {'class': 'PreventCombatDamageByChosenEffect', 'filter': self.filter(args[0])}
        if cls == 'PreventAllDamageToSourceEffect' and [a.strip() for a in args] == ['Duration.EndOfTurn']:
            return {'class': cls}
        if cls == 'TapAllEffect' and len(args) == 1:
            return {'class': cls, 'filter': self.filter(args[0])}
        if cls == 'RevealCardsFromLibraryUntilEffect' and len(args) == 3 \
                and [a.strip() for a in args[1:]] == ['PutCards.HAND', 'PutCards.BOTTOM_RANDOM']:
            return {'class': 'RevealUntilToHandEffect', 'filter': self.filter(args[0])}
        if cls == 'GainControlAllControlledTargetEffect' and len(args) == 1:
            return {'class': cls, 'filter': self.filter(args[0])}
        if cls == 'CantAttackTargetEffect' and [a.strip() for a in args] == ['Duration.EndOfTurn']:
            return {'class': 'GainAbilityTargetEffect', 'ability': {'class': 'CantAttackAbility'}, 'duration': 'EndOfTurn'}
        if cls in ('TapEnchantedEffect', 'DontUntapInControllersUntapStepEnchantedEffect') and len(args) <= 1:
            return {'class': cls}
        if cls in ('DiscardHandDrawSameNumberSourceEffect', 'MustBeBlockedByAtLeastOneSourceEffect') and not args:
            return {'class': cls}
        if cls == 'MustBeBlockedByAtLeastOneSourceEffect' and [a.strip() for a in args] == ['Duration.WhileOnBattlefield']:
            return {'class': cls}
        # "… for as long as you control this" (taken as for good)
        if cls == 'GainAbilityTargetEffect' and len(args) == 2 and args[1].strip() == 'Duration.WhileControlled':
            return self.effect('new GainAbilityTargetEffect(' + args[0] + ',Duration.WhileOnBattlefield)')
        # "if it's an artifact, it gains …": worked out as it resolves
        if cls == 'ConditionalContinuousEffect' and len(args) == 3 \
                and re.fullmatch(r'new GainAbilityTargetEffect\(.*,Duration\.EndOfTurn\)', canonical(args[0].strip()), re.S) \
                and (l := re.fullmatch(r'new LockedInCondition\((.*)\)', args[1].strip(), re.S)):
            return {'class': 'ConditionalOneShotEffect', 'effect': self.effect(args[0]), 'condition': self.condition(l.group(1))}
        # "target creature can't block this turn"
        if cls == 'CantBlockTargetEffect' and [a.strip() for a in args] == ['Duration.EndOfTurn']:
            return {'class': 'GainAbilityTargetEffect', 'ability': {'class': 'CantBlockAbility'}, 'duration': 'EndOfTurn'}
        # "target creature can't be blocked by <filter> this turn"
        if cls == 'CantBeBlockedByAllTargetEffect' and len(args) == 2 and args[1].strip() == 'Duration.EndOfTurn':
            return {'class': 'GainAbilityTargetEffect', 'ability': {'class': 'CantBeBlockedByAbility', 'filter': self.filter(args[0])},
                    'duration': 'EndOfTurn'}
        if cls == 'SearchLibraryGraveyardPutInHandEffect' and 1 <= len(args) <= 3 \
                and all(a.strip() in ('true', 'false') for a in args[1:]):
            return {'class': cls, 'filter': self.filter(args[0]), 'both': len(args) >= 2 and args[1].strip() == 'true'}
        if cls == 'ScryTargetEffect' and len(args) == 1:
            return {'class': cls, 'amount': self.value('amount', args[0])}
        if cls == 'ExileTopXMayPlayUntilEffect' and len(args) in (2, 3) \
                and args[-1].strip() in ('Duration.EndOfTurn', 'Duration.UntilEndOfYourNextTurn'):
            return {'class': cls, 'amount': self.value('amount', args[0]),
                    'choose_one': len(args) == 3 and args[1].strip() == 'true',
                    'duration': args[-1].strip().split('.')[1]}
        if cls == 'CastAsThoughItHadFlashAllEffect' and len(args) == 3 and args[0].strip() == 'Duration.WhileOnBattlefield' \
                and args[2].strip() == 'false':
            return {'class': cls, 'filter': self.filter(args[1])}
        if cls == 'CastAsThoughItHadFlashAllEffect' and len(args) == 2 and args[0].strip() == 'Duration.UntilYourNextTurn':
            return {'class': 'CastAsThoughItHadFlashUntilYourNextTurnEffect', 'filter': self.filter(args[1])}
        if cls == 'ConniveSourceEffect' and args in (['"{this}"'], []):
            return {'class': cls}
        if cls == 'BasicManaEffect' and len(args) == 1:
            a = args[0].strip()
            n = re.fullmatch(r'newMana\((\d+),(\d+),(\d+),(\d+),(\d+),0,0,(\d+)\)', a.replace(' ', ''))
            if n:
                v = [int(x) for x in n.groups()]
                return {'class': cls, 'colored': v[:5], 'colorless': v[5]}
            n = re.fullmatch(r'Mana\.(White|Blue|Black|Red|Green|Colorless)Mana\((\d+)\)', a)
            if n:
                colored = [0] * 5
                if n.group(1) == 'Colorless':
                    return {'class': cls, 'colored': colored, 'colorless': int(n.group(2))}
                colored[['White', 'Blue', 'Black', 'Red', 'Green'].index(n.group(1))] = int(n.group(2))
                return {'class': cls, 'colored': colored, 'colorless': 0}
        if cls == 'CreateTokenControllerTargetEffect' and len(args) == 3 \
                and (m := re.fullmatch(r'new (\w+Token)\(\)', args[0].strip())) and re.fullmatch(r'\d+', args[1].strip()):
            return {'class': cls, 'token': token(m.group(1)), 'amount': int(args[1]), 'tapped': args[2].strip() == 'true'}
        if cls == 'BoostOpponentsEffect' and len(args) == 3 and re.fullmatch(r'-?\d+', args[0].strip()) \
                and re.fullmatch(r'-?\d+', args[1].strip()):
            return {'class': 'BoostAllEffect', 'power': int(args[0]), 'toughness': int(args[1]),
                    'duration': self.value('duration', args[2].strip()),
                    'filter': [CREATURE, {'p': 'controller', 'who': 'opponent'}], 'exclude_source': False}
        if cls == 'SpellCostReductionForEachSourceEffect' and len(args) == 2 and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'per': int(args[0]), 'amount': self.amount(args[1])}
        if cls == 'ReturnCardChosenFromGraveyardEffect' and len(args) == 3 and args[2].strip() == 'PutCards.HAND' \
                and args[0].strip() in ('true', 'false'):
            return {'class': cls, 'filter': self.filter(args[1]), 'optional': args[0].strip() == 'true'}
        if cls == 'RevealLibraryPickControllerEffect' and len(args) in (5, 6) and re.fullmatch(r'\d+', args[0].strip()) \
                and re.fullmatch(r'\d+', args[1].strip()) and [a.strip() for a in args[3:5]] == ['PutCards.BATTLEFIELD', 'PutCards.GRAVEYARD'] \
                and (len(args) == 5 or args[5].strip() in ('true', 'false')):
            return {'class': cls, 'amount': int(args[0]), 'pick': int(args[1]), 'filter': self.filter(args[2]),
                    'optional': len(args) == 5 or args[5].strip() == 'true', 'to': 'BATTLEFIELD'}
        if cls == 'RevealLibraryPickControllerEffect' and len(args) in (5, 6) and re.fullmatch(r'\d+', args[0].strip()) \
                and re.fullmatch(r'\d+', args[1].strip()) and [a.strip() for a in args[3:5]] == ['PutCards.HAND', 'PutCards.GRAVEYARD'] \
                and (len(args) == 5 or args[5].strip() in ('true', 'false')):
            return {'class': cls, 'amount': int(args[0]), 'pick': int(args[1]), 'filter': self.filter(args[2]),
                    'optional': len(args) == 5 or args[5].strip() == 'true'}
        if cls == 'MillThenPutInHandEffect' and len(args) in (2, 3) and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'amount': int(args[0]), 'filter': self.filter(args[1]),
                    'optional': len(args) == 2 or args[2].strip() == 'true'}
        # "gets +p/+t as long as you control a <filter>" (Wild Nacatl)
        if cls == 'BoostSourceWhileControlsEffect' and len(args) == 3 and all(re.fullmatch(r'-?\d+', a.strip()) for a in args[1:]):
            return {'class': 'ConditionalContinuousEffect',
                    'effect': {'class': 'BoostSourceEffect', 'power': int(args[1]), 'toughness': int(args[2]),
                               'duration': 'WhileOnBattlefield'},
                    'condition': {'c': 'permanents', 'filter': self.filter(args[0]), 'cmp': 'OR_GREATER', 'n': 1}}
        if cls == 'SpellCostReductionSourceEffect' and len(args) == 1 and not re.fullmatch(r'\d+', args[0].strip()):
            return {'class': 'SpellCostReductionForEachSourceEffect', 'per': 1, 'amount': self.amount(args[0])}
        if cls == 'SpellCostReductionSourceEffect' and len(args) in (1, 2) and re.fullmatch(r'\d+', args[0].strip()):
            out = {'class': cls, 'amount': int(args[0])}
            if len(args) == 2:
                out['condition'] = self.condition(args[1])
            return out
        # the "additional" flag only changes the text
        # Rest in Peace's: any player's cards, and tokens
        if cls == 'GraveyardFromAnywhereExileReplacementEffect' and [a.strip() for a in args] == ['false', 'true']:
            return {'class': cls}
        if cls == 'ReturnFromGraveyardToBattlefieldWithCounterTargetEffect' and len(args) in (1, 2) \
                and (len(args) == 1 or args[0].strip() in ('true', 'false')):
            return {'class': 'ReturnFromGraveyardToBattlefieldTargetEffect', 'tapped': False,
                    'counters': self.value('counter', args[-1].strip())}
        if cls == 'AddCountersAllEffect' and len(args) == 3:
            return {'class': cls, 'counter': self.value('counter', args[0].strip()), 'amount': self.amount(args[1]),
                    'filter': self.filter(args[2].strip())}
        if cls == 'AddCountersSourceEffect' and len(args) == 2:
            return {'class': cls, 'counter': self.value('counter', args[0].strip()), 'amount': self.amount(args[1])}
        # investigate: create a Clue
        if cls == 'InvestigateEffect' and len(args) <= 1 and args != ['true'] and args != ['false']:
            return {'class': 'CreateTokenEffect', 'token': token('ClueArtifactToken'),
                    'amount': self.value('amount', args[0]) if args else 1}
        # create a token, then (you may) attach this Equipment to it
        if cls == 'CreateTokenAttachSourceEffect' and len(args) in (1, 2, 3):
            return {'class': 'CreateTokenEffect', 'token': self.value('token', args[0].strip()), 'amount': 1,
                    'attach_source': len(args) == 3 and args[2].strip() == 'true'}
        if cls == 'CreateTokenEffect' and len(args) == 4 and [a.strip() for a in args[2:]] == ['false', 'true']:
            return {'class': cls, 'token': self.value('token', args[0].strip()),
                    'amount': self.value('amount', args[1].strip()), 'attacking': True}
        # created tapped
        if cls == 'CreateTokenEffect' and len(args) == 4 and [a.strip() for a in args[2:]] == ['true', 'false']:
            return {'class': cls, 'token': self.value('token', args[0].strip()),
                    'amount': self.value('amount', args[1].strip()), 'tapped': True}
        if cls == 'EntersBattlefieldWithXCountersEffect' and len(args) == 1:
            return {'class': 'AddCountersSourceEffect', 'counter': self.value('counter', args[0].strip()),
                    'amount': {'v': 'x'}}
        if cls == 'DoIfAnyNumberCostPaid' and len(args) == 2 and args[0].strip() == 'new IntrepidAdversaryEffect()':
            return {'class': cls, 'cost': [self.cost(args[1].strip())],
                    'effects': [{'class': 'AddCountersSourceEffect', 'counter': {'name': 'valor', 'amount': 1},
                                 'amount': {'v': 'x'}}],
                    'reflexive': 'Put that many valor counters on {this}.'}
        # a cost reduction that applies while a condition holds
        if cls == 'ConditionalCostModificationEffect' and len(args) == 3:
            e = self.effect(args[0].strip())
            if e.get('class') != 'SpellsCostReductionControllerEffect':
                raise Unsupported('conditional cost modification ' + e.get('class', ''))
            e['condition'] = self.value('condition', args[1].strip())
            return e
        if cls == 'SpellsCostReductionControllerEffect' and len(args) == 2 and re.fullmatch(r'\d+', args[1].strip()):
            return {'class': cls, 'filter': self.filter(args[0].strip()), 'amount': int(args[1])}
        # "it gains your choice of …": the choice is each ability's rule text
        if cls == 'GainsChoiceOfAbilitiesEffect' and len(args) >= 3 and args[0].strip() == 'GainsChoiceOfAbilitiesEffect.TargetType.Source':
            abilities = [self.ability(a.strip()) for a in args[1:]]
            names = []
            for a in args[1:]:
                n = re.fullmatch(r'(\w+)Ability\.getInstance\(\)', a.strip())
                if not n:
                    raise Unsupported('choice of ' + a)
                names.append(re.sub(r'(?<!^)([A-Z])', r' \1', n.group(1)).lower())
            return {'class': cls, 'abilities': abilities, 'names': names}
        if cls == 'ChooseCreatureEffect' and not args:
            return {'class': cls, 'filter': [{'p': 'type', 't': 'CREATURE'}, {'p': 'controller', 'who': 'you'},
                                             {'p': 'another'}]}
        if cls == 'RegenerateSourceEffect' and len(args) <= 1:
            return {'class': cls}
        if cls == 'CounterUnlessPaysEffect' and len(args) == 1 and not args[0].strip().startswith('new '):
            return {'class': cls, 'generic': self.value('amount', args[0].strip())}
        # a characteristic-defining power and toughness of any count (Pack Rat, Territorial Kavu)
        if cls == 'SetBasePowerToughnessSourceEffect' and len(args) == 1 and args[0].strip() not in COUNTS:
            return {'class': 'SetBasePowerToughnessAmountSourceEffect', 'amount': self.amount(args[0])}
        if cls == 'CounterTargetWithReplacementEffect' and args == ['PutCards.EXILED']:
            return {'class': 'CounterTargetExileEffect'}
        if cls == 'LoseAllAbilitiesTargetEffect' and args == ['Duration.EndOfTurn']:
            return {'class': cls}
        if cls in ('DontUntapInControllersUntapStepSourceEffect', 'UntapSourceEffect', 'RegenerateTargetEffect') and not args:
            return {'class': cls}
        if cls == 'CantGainLifeAllEffect' and args == ['Duration.EndOfTurn']:
            return {'class': 'CantGainLifeThisTurnEffect'}
        if cls == 'SetBasePowerSourceEffect' and len(args) == 1:
            return {'class': cls, 'amount': self.amount(args[0])}
        # created tapped and attacking
        if cls == 'CreateTokenEffect' and len(args) == 4 and [a.strip() for a in args[2:]] == ['true', 'true']:
            return {'class': cls, 'token': self.value('token', args[0].strip()),
                    'amount': self.value('amount', args[1].strip()), 'tapped_and_attacking': True}
        # it becomes the token's kind of creature in place of what it was (types and subtypes), keeping its abilities
        if cls == 'BecomesCreatureTargetEffect' and len(args) == 4 and [a.strip() for a in args[1:3]] == ['false', 'false']:
            return {'class': cls, 'token': self.creature_token(args[0]), 'duration': self.value('duration', args[3].strip())}
        if cls == 'BecomesCreatureSourceEffect' and len(args) == 3:
            retain = args[1].strip()
            if retain not in ('CardType.LAND', 'CardType.ARTIFACT', 'CardType.ENCHANTMENT', 'CardType.PLANESWALKER', 'null'):
                raise Unsupported('becomes creature retaining ' + retain)
            duration = self.value('duration', args[2].strip())
            if duration not in ('EndOfTurn', 'WhileOnBattlefield'):
                raise Unsupported('becomes creature ' + args[2])
            return {'class': cls, 'token': self.creature_token(args[0]), 'retain': retain != 'null',
                    'duration': duration}
        if cls == 'ExploreSourceEffect' and (not args or len(args) in (2, 3)):
            return {'class': cls, 'amount': self.value('amount', args[0]) if len(args) == 3 else 1}
        if cls == 'DiscardEachPlayerEffect' and len(args) <= 1:
            who = self.value('target_controller', args[0]) if args else 'any'
            return {'class': cls, 'amount': 1, 'who': who}
        if cls == 'DiscardEachPlayerEffect' and len(args) == 2 and args[1].strip() == 'false':
            return {'class': cls, 'amount': self.value('amount', args[0]), 'who': 'any'}
        if cls == 'ExileGraveyardAllTargetPlayerEffect' and args in ([], ['false']):
            return {'class': cls}
        # `new DamagePlayersEffect(Outcome.Damage, StaticValue.get(2), TargetController.OPPONENT)`
        if cls == 'DamagePlayersEffect' and len(args) == 3 and args[0].strip().startswith('Outcome.'):
            n = re.fullmatch(r'StaticValue\.get\((\d+)\)', args[1].strip())
            if n:
                return {'class': cls, 'amount': int(n.group(1)), 'controller': self.value('target_controller', args[2].strip())}
        # "at the beginning of the next end step, …": its effects then
        if cls == 'CreateDelayedTriggeredAbilityEffect' and len(args) == 2 and args[1].strip() == 'false':
            args = args[:1]
        if cls == 'CreateDelayedTriggeredAbilityEffect' and len(args) == 1:
            if args[0].strip() == 'new CopyNextSpellDelayedTriggeredAbility()':
                return {'class': 'CreateCopyNextSpellDelayedEffect'}
            if args[0].strip() == 'new AddCounterNextSpellDelayedTriggeredAbility()':
                return {'class': 'CreateNextCreatureSpellDelayedEffect', 'amount': 1,
                        'rule': 'When you next cast a creature spell this turn, that creature enters with an '
                                'additional +1/+1 counter on it.'}
            d = self.vars.get(args[0].strip()) or self.ability(args[0])
            if d.get('class') == 'NextUpkeepDelayed':
                return {'class': 'CreateNextUpkeepDelayedEffect', 'effects': d['effects'], 'rule': d['rule']}
            if d.get('class') != 'NextEndStepDelayed':
                raise Unsupported('delayed ability ' + args[0])
            return {'class': 'CreateNextEndStepDelayedEffect', 'effects': d['effects'], 'rule': d['rule']}
        if cls == 'UntapLandsEffect' and len(args) == 1 and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'amount': int(args[0])}
        if cls == 'PutIntoLibraryNFromTopTargetEffect' and len(args) == 1 and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': cls, 'n': int(args[0])}
        if cls == 'GetEmblemEffect' and len(args) == 1:
            n = re.fullmatch(r'new (\w+Emblem)\(\)', args[0].strip())
            if not n:
                raise Unsupported('emblem ' + args[0])
            return {'class': cls, 'emblem': emblem(n.group(1))}
        if cls == 'SacrificeOpponentsEffect' and len(args) in (1, 2):
            return {'class': cls, 'filter': self.filter(args[-1]),
                    'amount': self.value('amount', args[0]) if len(args) == 2 else 1}
        if cls == 'DiscardCardYouChooseTargetEffect' and not args:
            return {'class': cls, 'filter': []}
        if cls == 'DrawDiscardControllerEffect' and len(args) <= 1:
            args = ['1', '1'] + args
        if cls == 'LookLibraryAndPickControllerEffect':
            return self.look_library(args)
        # a text override as the last argument (rule texts come from the card facts)
        # "choose a color": the permanents of the filter gain protection from it until end of turn
        if cls == 'GainProtectionFromColorAllEffect' and len(args) == 2 and args[0].strip() == 'Duration.EndOfTurn':
            return {'class': cls, 'filter': self.filter(args[1])}
        if cls == 'GainAbilityAllEffect' and len(args) == 4 and args[3].strip().startswith('"'):
            args = args[:3]
        if cls not in EFFECTS:
            raise Unsupported('effect ' + cls)
        return self.build(cls, args, EFFECTS[cls], 'effect')

    def library_target(self, expr):
        """`new TargetCardInLibrary([[min,] max,] [filter])` -> filter, min, max"""
        obj = self.new_expr(expr)
        if isinstance(obj, dict) and 'library_target' in obj:
            return obj['library_target']
        if isinstance(obj, dict) or obj[0] != 'TargetCardInLibrary':
            raise Unsupported('library target ' + expr)
        cls, args = obj
        args = [a.strip() for a in args]
        ints = [int(a) for a in args if re.fullmatch(r'\d+', a)]
        rest = [a for a in args if not re.fullmatch(r'\d+', a)]
        if len(rest) > 1 or len(ints) > 2:
            raise Unsupported('library target ' + expr)
        filt = self.filter(rest[0]) if rest else []
        lo, hi = (ints * 2)[:2] if len(ints) == 1 else (ints or [1, 1])
        return filt, lo, hi

    def search_library(self, cls, args):
        """The `SearchLibraryPutIn…Effect`s as one `SearchLibraryEffect`"""
        out = {'class': 'SearchLibraryEffect'}
        if cls == 'SearchLibraryPutInGraveyardEffect' and len(args) == 1:
            out.update(filter=[], min=1, max=1, to='GRAVEYARD')
            return out
        if cls == 'SearchLibraryPutInPlayTargetControllerEffect':
            if len(args) == 1:
                args = ['new TargetCardInLibrary(StaticFilters.FILTER_CARD_BASIC_LAND)'] + args
            if len(args) not in (2, 4) or args[1] not in ('true', 'false'):
                raise Unsupported('effect ' + cls)
            out['filter'], out['min'], out['max'] = self.library_target(args[0])
            out.update(to='BATTLEFIELD_TAPPED' if args[1] == 'true' else 'BATTLEFIELD', target_controller=True)
            return out
        if not args or any(a not in ('true', 'false') for a in args[1:]):
            raise Unsupported('effect ' + cls)
        out['filter'], out['min'], out['max'] = self.library_target(args[0])
        flags = [a == 'true' for a in args[1:]]
        if cls == 'SearchLibraryPutInPlayEffect':
            # (target, tapped, textThatCard, optional)
            out['to'] = 'BATTLEFIELD_TAPPED' if flags[:1] == [True] else 'BATTLEFIELD'
            out['optional'] = len(flags) == 3 and flags[2]
        elif cls == 'SearchLibraryPutInHandEffect' and len(flags) in (1, 2):
            out['to'] = 'HAND'
        else:
            raise Unsupported('effect ' + cls)
        return out

    def look_library(self, args):
        """`LookLibraryAndPickControllerEffect`'s overloads: (n, pick, picked, looked[, upTo]) takes any cards;
        (n, pick, filter, picked, looked[, optional]) asks first when optional (the default) and is "up to" for more
        than one"""
        args = [a.strip() for a in args]
        out = {'class': 'LookLibraryAndPickControllerEffect'}
        if re.fullmatch(r'\d+', args[0]):
            out['look'] = int(args[0])
        elif args[0] in COUNTS:
            out['look_count'] = COUNTS[args[0]]
        else:
            raise Unsupported('argument ' + args[0])
        if args[1] == 'Integer.MAX_VALUE' and 'look' in out:
            args[1] = str(out['look'])
        if not re.fullmatch(r'\d+', args[1]):
            raise Unsupported('argument ' + args[1])
        out['pick'] = int(args[1])
        rest = args[2:]
        puts = [a for a in rest if a.startswith('PutCards.')]
        if len(puts) != 2 or any(p[len('PutCards.'):] not in PUT_CARDS for p in puts):
            raise Unsupported('LookLibraryAndPickControllerEffect(' + ','.join(args) + ')')
        out['picked'], out['looked'] = (p[len('PutCards.'):] for p in puts)
        if rest[0].startswith('PutCards.'):
            # any cards; "up to" when asked
            out['up_to'] = len(rest) == 3 and rest[2] == 'true'
            if len(rest) not in (2, 3):
                raise Unsupported('LookLibraryAndPickControllerEffect(' + ','.join(args) + ')')
        else:
            out['filter'] = self.filter(rest[0])
            out['optional'] = not (len(rest) == 4 and rest[3] == 'false')
            out['up_to'] = out['pick'] > 1
        return out

    def target(self, expr):
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            return obj
        cls, args = obj
        if cls in PLAYER_TARGETS and not args:
            return {'class': cls}
        # Consign to Memory: "triggered ability or colorless spell"
        if cls == 'TargetStackObject' and len(args) == 1 and 'ConsignToMemoryPredicate' in str(self.filters.get(args[0].strip())):
            return {'class': 'TargetTriggeredAbilityOrColorlessSpell'}
        # X divided among any number of creatures (one point each at least)
        if cls == 'TargetAnyTargetAmount' and len(args) == 1:
            return {'class': 'TargetAnyTarget', 'min': 1, 'max': 99, 'distribute': self.amount(args[0])}
        if cls == 'TargetCreaturePermanentAmount' and len(args) == 1:
            return {'class': 'TargetPermanent', 'filter': [dict(CREATURE)], 'min': 1, 'max': 99,
                    'distribute': self.amount(args[0])}
        if cls == 'TargetCreaturePermanentAmount' and len(args) == 2:
            return {'class': 'TargetPermanent', 'filter': [dict(CREATURE)] + self.filter(args[1]), 'min': 1, 'max': 99,
                    'distribute': self.amount(args[0])}
        if cls == 'TargetPermanentOrPlayer' and len(args) == 4 and self.filters.get(args[2].strip()) == ANY_TARGET_NOT_FIRST \
                and re.fullmatch(r'\d+', args[0].strip()) and re.fullmatch(r'\d+', args[1].strip()) and args[3].strip() == 'false':
            return {'class': 'TargetAnyTarget', 'min': int(args[0]), 'max': int(args[1]), 'not_first': True}
        if cls == 'TargetPermanentOrPlayer' and len(args) == 1 and self.filters.get(args[0].strip()) == ANY_OTHER_TARGET:
            return {'class': 'TargetAnyTarget', 'another': True}
        if cls == 'TargetSpellOrPermanent' and [a.strip() for a in args[:2] + args[3:]] == ['1', '1', 'false'] \
                and self.filters.get(args[2].strip()) is SPELL_OR_CREATURE:
            return {'class': cls, 'filter': [dict(CREATURE)]}
        if cls == 'TargetAnyTarget' and len(args) == 2 and all(re.fullmatch(r'\d+', a.strip()) for a in args):
            return {'class': cls, 'min': int(args[0]), 'max': int(args[1])}
        if cls not in TARGET_CLASSES:
            raise Unsupported('target ' + expr)
        kind, preds = TARGET_CLASSES[cls]
        out = {'class': kind, 'filter': [dict(p) for p in preds]}
        ints = [int(a) for a in args if re.fullmatch(r'\d+', a.strip())]
        rest = [a for a in args if not re.fullmatch(r'\d+', a.strip())]
        if rest and rest[-1].strip() == 'false':
            rest = rest[:-1]
        if len(rest) > 1:
            raise Unsupported('target ' + expr)
        if rest:
            # a card in your (an opponent's) graveyard stays one, whatever the filter
            keep = preds if cls in ('TargetCardInYourGraveyard', 'TargetCardInOpponentsGraveyard') else []
            out['filter'] = [dict(p) for p in keep] + self.filter(rest[0])
        if kind == 'TargetSpell':
            if ints:
                raise Unsupported('target ' + expr)
        elif len(ints) == 1:
            out['min'] = out['max'] = ints[0]
        elif len(ints) == 2:
            out['min'], out['max'] = ints
        elif ints:
            raise Unsupported('target ' + expr)
        if cls == 'TargetCardInASingleGraveyard':
            out['single'] = True
        return out

    def ability(self, expr):
        # an evasion static is a static
        m = re.fullmatch(r'new SimpleEvasionAbility\((.*)\)', expr.strip(), re.S)
        if m:
            a = self.ability('new SimpleStaticAbility(' + m.group(1) + ')')
            a['java'] = 'SimpleEvasionAbility'
            return a
        # enters with an effect if …: the overload with "optional" second
        m = re.fullmatch(r'new EntersBattlefieldAbility\((.*)\)', expr.strip(), re.S)
        if m and len(a := split_args(m.group(1))) == 5 and a[1].strip() == 'false':
            return self.ability('new EntersBattlefieldAbility(' + ','.join([a[0]] + a[2:]) + ')')
        # the overload with its zone first: the battlefield's is the two-argument one
        m = re.fullmatch(r'new ActivateAsSorceryActivatedAbility\(Zone\.BATTLEFIELD,(.*)\)', expr.strip(), re.S)
        if m and len(split_args(m.group(1))) == 2:
            return self.ability('new ActivateAsSorceryActivatedAbility(' + m.group(1) + ')')
        # `….withInterveningIf(condition)`: the "if" of a triggered ability
        m = re.fullmatch(r'(.*\))\s*\.withInterveningIf\((\w+(?:\.instance|\.ONCE|\.WHITE|\.BLUE|\.BLACK|\.RED|\.GREEN)?)\)', self.strip_setters(expr), re.S)
        if m:
            a = self.ability(m.group(1))
            # Old-Growth Troll "if it was a creature": the Aura it returns as doesn't die
            if m.group(2) == 'OldGrowthTrollCondition.instance' and a.get('on') == {'e': 'dies'}:
                a['on'] = {'e': 'dies_if_creature'}
                return a
            a['condition'] = self.value('condition', m.group(2))
            return a
        # `….withTriggerCondition(condition)`: checked only as it triggers
        m = re.fullmatch(r'(.*\))\s*\.withTriggerCondition\((\w+(?:\.instance|\.TRUE)?)\)', self.strip_setters(expr), re.S)
        if m:
            a = self.ability(m.group(1))
            a['trigger_condition'] = self.value('condition', m.group(2))
            return a
        # `….setTriggersLimitEachTurn(1)`
        m = re.fullmatch(r'(.*\))\s*\.setTriggersLimitEachTurn\((\d+)\)', self.strip_setters(expr), re.S)
        # "whenever one or more … you control enter … only once each turn": the first to enter each turn triggers it,
        # alone or in a batch
        if m and m.group(2) == '1' \
                and (o := re.fullmatch(r'new EntersBattlefieldOneOrMoreTriggeredAbility\((.*)\)', m.group(1).strip(), re.S)):
            args = split_args(o.group(1))
            if len(args) == 3 and args[2].strip() == 'TargetController.YOU':
                a = triggered({'class': 'EntersBattlefieldAllTriggeredAbility',
                               'java': 'EntersBattlefieldOneOrMoreTriggeredAbility',
                               'effects': [self.effect(args[0])], 'filter': self.filter(args[1]) + [YOU],
                               'optional': False, 'set_target_pointer': False})
                a['limit_per_turn'] = 1
                a['on']['batch'] = True
                return a
        if m:
            a = self.ability(m.group(1))
            if a.get('class') != 'TriggeredAbility':
                raise Unsupported('a trigger limit on ' + a.get('class', '?'))
            a['limit_per_turn'] = int(m.group(2))
            return a
        # `….setTiming(TimingRule.SORCERY)` on an ability from the graveyard
        m = re.fullmatch(r'(.*\))\s*\.setTiming\(TimingRule\.SORCERY\)', self.strip_setters(expr), re.S)
        if m and balanced(m.group(1)):
            a = self.ability(m.group(1))
            if a.get('class') != 'GraveyardActivatedAbility':
                raise Unsupported('sorcery timing on ' + a.get('class', '?'))
            a['sorcery'] = True
            return a
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            return obj
        cls, args = obj
        if cls in KEYWORDS and not args:
            return {'class': cls}
        # "{cost}: …" from your graveyard, at any time
        if cls == 'SimpleActivatedAbility' and len(args) == 3 and args[0].strip() == 'Zone.GRAVEYARD':
            return {'class': 'GraveyardActivatedAbility', 'java': cls, 'effects': [self.effect(args[1])],
                    'cost': [self.cost(args[2])]}
        # "{cost}: … Activate only if …", from your graveyard
        if cls == 'ActivateIfConditionActivatedAbility' and len(args) == 4 and args[0].strip() == 'Zone.GRAVEYARD':
            return {'class': 'GraveyardActivatedAbility', 'java': cls, 'effects': [self.effect(args[1])],
                    'cost': [self.cost(args[2])], 'condition': self.condition(args[3])}
        # "{cost}: … Activate only if …" (a Case's solved ability): any number of times, while the condition holds
        if cls == 'ActivateIfConditionActivatedAbility' and len(args) == 3:
            return {'class': 'LimitedTimesPerTurnActivatedAbility', 'java': cls, 'effects': [self.effect(args[0])],
                    'cost': [self.cost(args[1])], 'limit': 4294967295, 'condition': self.condition(args[2])}
        # Mobilize N: whenever this attacks, N 1/1 red Warriors tapped and attacking, sacrificed at the next end step
        if cls == 'MobilizeAbility' and len(args) == 1 and args[0].strip().isdigit():
            a = self.ability('new AttacksTriggeredAbility(new CreateTokenEffect(new RedWarriorToken(),'
                             + args[0].strip() + ',true,true),false)')
            a['java'] = cls
            a['effects'][0]['sacrifice_at_next_end_step'] = True
            return a
        if cls == 'NinjutsuAbility' and len(args) == 1 and re.fullmatch(r'"\{[^"]*\}"', args[0].strip()):
            return {'class': cls, 'java': cls, 'effects': [{'class': 'NinjutsuEffect'}],
                    'cost': [{'class': 'ManaCost', 'cost': args[0].strip()[1:-1]},
                             {'class': 'ReturnAttackerToHandTargetCost'}]}
        # `new DelveAbility(false)`: the flag only keeps the exiled cards linked to the source
        # affinity for a land type: the spell costs {1} less for each one you control
        if cls == 'SunburstAbility' and [a.strip() for a in args] == ['this']:
            return {'class': 'SunburstAbility', 'java': cls}
        if cls == 'AffinityForArtifactsAbility' and not args:
            return {'class': 'SimpleStaticAbility', 'java': cls, 'effects': [{
                'class': 'SpellCostReductionForEachSourceEffect', 'per': 1,
                'amount': {'v': 'permanents', 'filter': [{'p': 'type', 't': 'ARTIFACT'}, {'p': 'controller', 'who': 'you'}]}}]}
        if cls == 'AffinityAbility' and len(args) == 1 and (n := re.fullmatch(r'AffinityType\.(FORESTS|ISLANDS|SWAMPS|MOUNTAINS|PLAINS)', args[0].strip())):
            land = {'FORESTS': 'Forest', 'ISLANDS': 'Island', 'SWAMPS': 'Swamp', 'MOUNTAINS': 'Mountain', 'PLAINS': 'Plains'}[n.group(1)]
            return {'class': 'SimpleStaticAbility', 'java': cls, 'effects': [{
                'class': 'SpellCostReductionForEachSourceEffect', 'per': 1,
                'amount': {'v': 'permanents', 'filter': [{'p': 'subtype', 's': land}, {'p': 'controller', 'who': 'you'}]}}]}
        if cls == 'SimpleActivatedAbility' and len(args) == 2 and getattr(self, 'min_x', 0) \
                and args[1].strip() in self.vars and self.vars[args[1].strip()].get('class') == 'ManaCost':
            return {'class': cls, 'java': cls, 'effects': [self.effect(args[0])], 'cost': [self.cost(args[1])],
                    'min_x': self.min_x}
        # equip: as a sorcery, attach this to target creature you control
        if cls == 'EquipAbility' and len(args) in (1, 2, 3, 4):
            if len(args) <= 2 and re.fullmatch(r'\d+', args[0].strip()):
                cost = {'class': 'ManaCost', 'cost': '{' + args[0].strip() + '}'}
            elif len(args) == 3 and args[0].strip().startswith('Outcome.') and args[2].strip() in ('true', 'false'):
                cost = self.cost(args[1])
            elif len(args) == 4 and args[0].strip().startswith('Outcome.') \
                    and args[2].strip() == 'new TargetControlledCreaturePermanent()' and args[3].strip() in ('true', 'false'):
                cost = self.cost(args[1])
            else:
                raise Unsupported('equip ' + ','.join(args))
            return {'class': cls, 'java': cls, 'effects': [{'class': 'EquipEffect'}], 'cost': [cost],
                    'targets': [{'class': 'TargetPermanent', 'filter': [CREATURE, YOU]}]}
        if cls == 'NykthosShrineToNyxManaAbility' and not args:
            return {'class': cls, 'java': cls, 'cost': [{'class': 'ManaCost', 'cost': '{2}'}]}
        if cls == 'EntersBattlefieldTappedAsItEntersChooseColorAbility' and not args:
            return {'class': 'AsEntersBattlefieldAbility', 'java': cls, 'effects': [{'class': 'EntersTappedChooseColorEffect'}]}
        if cls == 'SimpleManaAbility' and len(args) == 3 and args[0].strip() == 'Zone.BATTLEFIELD' \
                and args[2].strip() == 'new TapSourceCost()':
            if args[1].strip() == 'new AddManaChosenColorEffect()':
                return {'class': 'ChosenColorManaAbility', 'java': cls, 'amount': 1}
            if (n := re.fullmatch(r'new AddConditionalManaChosenColorEffect\((\d+),new SunkenCitadelManaBuilder\(\)\)', canonical(args[1]))):
                return {'class': 'ChosenColorManaAbility', 'java': cls, 'amount': int(n.group(1)), 'land_abilities': True}
        if cls == 'ProtectionAbility' and len(args) == 1:
            return {'class': cls, 'filter': self.filter(args[0])}
        if cls == 'DelveAbility' and args in ([], ['false']):
            return {'class': cls}
        # `new EnchantAbility(auraTarget)`: what it can enchant is the spell's target
        if cls == 'EnchantAbility' and len(args) == 1:
            return {'class': cls}
        # `new MenaceAbility(false)`: the flag only changes the reminder text
        if cls == 'MenaceAbility' and args in (['false'], ['true']):
            return {'class': cls}
        if cls == 'SimpleStaticAbility' and len(args) == 2 and (args[0] in ('Zone.ALL', 'Zone.BATTLEFIELD', 'Zone.COMMAND') or (args[0] == 'Zone.STACK' and 'CantBeCounteredSourceEffect' in args[1])):
            args = args[1:]
        # a Class: its levels, gained as a sorcery one after another, and the abilities each level adds
        # casualty N: as you cast it, you may sacrifice a creature with power N or greater; when you do, copy it
        if cls == 'CasualtyAbility' and len(args) == 1 and args[0].strip().isdigit():
            return {'class': cls, 'java': cls, 'n': int(args[0].strip())}
        # teamwork N: as you cast it, you may tap creatures with total power N or more (kept as the kicked flag)
        if cls == 'TeamworkAbility' and len(args) == 1 and args[0].strip().isdigit():
            return {'class': cls, 'java': cls, 'n': int(args[0].strip())}
        # foretell: during your turn, pay {2} and exile it face down; cast it on a later turn for the foretell cost
        if cls == 'ForetellAbility' and len(args) == 2 and args[0].strip() == 'this' \
                and re.fullmatch(r'"(\{[^"]*\})"', args[1].strip()):
            return {'class': cls, 'java': cls, 'cost': args[1].strip()[1:-1]}
        if cls == 'BargainAbility' and not args:
            return {'class': cls, 'java': cls}
        # plot: as a sorcery, pay the cost and exile it from your hand; cast it free as a sorcery on a later turn
        if cls == 'PlotAbility' and len(args) == 1 and re.fullmatch(r'"(\{[^"]*\})+"', args[0].strip()):
            return {'class': cls, 'java': cls, 'cost': args[0].strip()[1:-1]}
        if cls == 'AttacksEachCombatStaticAbility' and not args:
            return {'class': 'SimpleStaticAbility', 'java': cls, 'effects': [{'class': 'AttacksIfAbleSourceEffect'}]}
        # flashback for a cost that isn't mana (Lava Dart: sacrifice a Mountain)
        if cls == 'FlashbackAbility' and len(args) == 2 and args[0].strip() == 'this' \
                and re.fullmatch(r'new (Sacrifice|Discard|Exile|PayLife|Tap)\w*Cost\(.*\)', args[1].strip(), re.S):
            return {'class': cls, 'java': cls, 'cost': '', 'costs': [self.cost(args[1])]}
        if cls == 'LeylineAbility' and not args:
            return {'class': cls, 'java': cls}
        if cls == 'ClassReminderAbility' and not args:
            return {'class': cls, 'java': cls}
        if cls == 'ClassLevelAbility' and len(args) == 2 and args[0].strip().isdigit() \
                and re.fullmatch(r'"(\{[^"]*\})+"', args[1].strip()):
            n = int(args[0].strip())
            return {'class': cls, 'java': cls, 'level': n, 'effects': [{'class': 'SetClassLevelEffect', 'level': n}],
                    'cost': [{'class': 'ManaCost', 'cost': args[1].strip()[1:-1]}]}
        if cls == 'BecomesClassLevelTriggeredAbility' and len(args) == 2 and args[1].strip().isdigit():
            return {'class': 'TriggeredAbility', 'java': cls, 'on': {'e': 'class_level', 'n': int(args[1].strip())},
                    'effects': [self.effect(args[0])], 'optional': False}
        # Serra Ascendant: +5/+5 and flying while you have 30 or more life
        if cls == 'SimpleStaticAbility' and args == ['new SerraAscendantEffect()']:
            thirty = {'c': 'life_compare', 'who': 'you', 'cmp': 'OR_GREATER', 'n': 30}
            return {'class': 'SimpleStaticAbility', 'java': cls, 'effects': [
                {'class': 'ConditionalContinuousEffect', 'condition': thirty,
                 'effect': {'class': 'BoostSourceEffect', 'power': 5, 'toughness': 5, 'duration': 'WhileOnBattlefield'}},
                {'class': 'ConditionalContinuousEffect', 'condition': thirty,
                 'effect': {'class': 'GainAbilitySourceEffect', 'ability': {'class': 'FlyingAbility'},
                            'duration': 'WhileOnBattlefield'}}]}
        # Scion of Draco: each creature you control of a color has that color's keyword
        if cls == 'SimpleStaticAbility' and args == ['new ScionOfDracoEffect()']:
            return {'class': 'SimpleStaticAbility', 'java': cls, 'effects': [
                {'class': 'GainAbilityControlledEffect', 'ability': {'class': k}, 'duration': 'WhileOnBattlefield',
                 'filter': [{'p': 'type', 't': 'CREATURE'}, {'p': 'color', 'c': c}], 'exclude_source': False}
                for c, k in (('W', 'VigilanceAbility'), ('U', 'HexproofAbility'), ('B', 'LifelinkAbility'),
                             ('R', 'FirstStrikeAbility'), ('G', 'TrampleAbility'))]}
        if cls == 'SimpleStaticAbility' and len(args) == 1 \
                and (g := re.fullmatch(r'new GainClassAbilitySourceEffect\((.*)\)', args[0].strip(), re.S)):
            inner, level = split_args(g.group(1))
            gate = {'c': 'class_level', 'n': int(level.strip())}
            # a static effect the level adds: the effect under that condition
            if re.fullmatch(r'new \w+Effect\(.*\)', inner.strip(), re.S):
                e = self.effect(inner)
                # a boost the level adds: the boost while the class is at that level
                if e.get('class') == 'BoostControlledEffect':
                    return {'class': 'SimpleStaticAbility', 'java': cls,
                            'effects': [{'class': 'ConditionalContinuousEffect', 'effect': e, 'condition': gate}]}
                if e.get('class') not in ('SpellsCostReductionControllerEffect', 'ArtistsTalentEffect') or e.get('condition'):
                    raise Unsupported('class level effect ' + inner)
                e['condition'] = gate
                return {'class': 'SimpleStaticAbility', 'java': cls, 'effects': [e]}
            a = self.ability(inner)
            if a.get('class') != 'TriggeredAbility' or a.get('trigger_condition'):
                raise Unsupported('class level ability ' + inner)
            a['trigger_condition'] = gate
            a['java'] = cls
            return a
        if cls == 'EntersBattlefieldAbility' and args == ['new MockingbirdEffect()', 'true']:
            return {'class': cls, 'effects': [{'class': 'MockingbirdEffect'}]}
        if cls == 'EntersBattlefieldAbility' and len(args) == 2 and re.fullmatch(r'"[^"]*"', args[1].strip()):
            return {'class': cls, 'effects': [self.effect(args[0])]}
        if cls == 'EntersBattlefieldTappedUnlessAbility' and len(args) in (1, 2):
            return {'class': 'EntersBattlefieldAbility', 'java': cls, 'effects': [
                {'class': 'ConditionalOneShotEffect', 'otherwise': {'class': 'TapSourceEffect'},
                 'condition': self.condition(args[0])}]}
        if cls in STEP_TRIGGERS and len(args) == 4 and args[0].strip() == 'Zone.BATTLEFIELD':
            args = args[1:]
        # a step trigger of a card in its owner's graveyard (Arclight Phoenix)
        if cls == 'LandfallAbility' and len(args) in (2, 3) and args[0].strip() == 'Zone.GRAVEYARD':
            a = self.ability('new LandfallAbility(' + ','.join(args[1:]) + ')')
            a['on']['graveyard'] = True
            return a
        if cls in STEP_TRIGGERS and len(args) == 4 and args[0].strip() == 'Zone.GRAVEYARD':
            a = self.ability('new ' + cls + '(' + ','.join(args[1:]) + ')')
            a['on']['graveyard'] = True
            return a
        if cls == 'EntersBattlefieldAllTriggeredAbility' and len(args) == 4 and args[0].strip() == 'Zone.BATTLEFIELD' \
                and args[3].strip() in ('true', 'false'):
            args = args[1:]
        if cls == 'EntersBattlefieldAllTriggeredAbility' and len(args) == 5 and args[0].strip() == 'Zone.BATTLEFIELD' \
                and args[4].strip() in ('SetTargetPointer.PERMANENT', 'SetTargetPointer.PLAYER', 'SetTargetPointer.NONE') \
                and args[3].strip() in ('true', 'false'):
            return triggered({'class': cls, 'effects': [self.effect(args[1])], 'filter': self.filter(args[2]),
                              'optional': args[3].strip() == 'true',
                              'set_target_pointer': args[4].strip() == 'SetTargetPointer.PERMANENT',
                              'set_target_pointer_player': args[4].strip() == 'SetTargetPointer.PLAYER'})
        if cls in STEP_TRIGGERS and args and not args[0].strip().startswith('TargetController.'):
            args = ['TargetController.YOU'] + args
        if cls == 'SpellCastControllerTriggeredAbility' and len(args) == 2:
            args = [args[0], 'StaticFilters.FILTER_SPELL_A', args[1]]
        # opus: a cast trigger for instants and sorceries, its extra effect if five or more mana was spent on the spell
        if cls == 'OpusAbility' and len(args) == 4 and args[2].strip() == 'null' and args[3].strip() == 'false':
            a = self.ability('new SpellCastControllerTriggeredAbility(' + args[0]
                             + ', StaticFilters.FILTER_SPELL_AN_INSTANT_OR_SORCERY, false)')
            a['effects'].append({'class': 'ConditionalOneShotEffect', 'effect': self.effect(args[1]),
                                 'condition': {'c': 'opus'}})
            a['java'] = cls
            return a
        if cls == 'CantAttackOrBlockAloneAbility' and not args:
            return {'class': 'SimpleStaticAbility', 'java': cls,
                    'effects': [{'class': 'CantAttackAloneSourceEffect'}, {'class': 'CantBlockAloneSourceEffect'}]}
        # Devourer of Destiny: revealed from the opening hand, its first-upkeep trigger (the host asks it)
        if cls == 'ChancellorAbility' and len(args) == 2 and args[0].strip() == 'new DevourerOfDestinyTriggeredAbility()':
            return {'class': 'ChancellorAbility', 'java': cls}
        # Gemstone Caverns: "{T}: Add {C}. If this has a luck counter on it, instead add one mana of any color."
        if cls == 'ConditionalManaAbility' and len(args) == 3 and 'SourceHasCounterCondition(CounterType.LUCK)' in args[1] \
                and args[2].strip() == 'new TapSourceCost()':
            return {'class': 'ColorlessManaAbility', 'java': cls, 'any_color_with': 'luck'}
        # Ugin's Labyrinth: "{T}: Add {C}. If a card is exiled with this, add {C}{C} instead."
        if cls == 'SimpleManaAbility' and len(args) == 3 and 'UginsLabyrinthCondition.instance' in args[1] \
                and args[2].strip() == 'new TapSourceCost()':
            return {'class': 'ColorlessManaAbility', 'java': cls, 'two_if_exiled': True}
        # Eldrazi Temple: "{T}: Add {C}{C}. Spend this mana only to cast colorless Eldrazi spells …"
        if cls == 'ConditionalColorlessManaAbility' and [a.strip() for a in args] == \
                ['new TapSourceCost()', '2', 'new EldraziTempleManaBuilder()']:
            return {'class': 'ConditionalManaAbility', 'java': cls, 'colorless': True, 'amount': 2, 'filter_abilities': True,
                    'filter': [{'p': 'colorless'}, {'p': 'subtype', 's': 'Eldrazi'}]}
        # adapt N: "{cost}: if this has no +1/+1 counters, put N on it"
        if cls == 'AdaptAbility' and len(args) == 2 and re.fullmatch(r'\d+', args[0].strip()) \
                and re.fullmatch(r'"[^"]*"', args[1].strip()):
            return {'class': 'SimpleActivatedAbility', 'java': cls,
                    'effects': [{'class': 'AdaptEffect', 'amount': int(args[0])}],
                    'cost': [{'class': 'ManaCost', 'cost': args[1].strip()[1:-1]}]}
        # "whenever one or more +1/+1 counters are put on this"
        if cls == 'OneOrMoreCountersAddedTriggeredAbility' and len(args) in (1, 2):
            return {'class': 'TriggeredAbility', 'java': cls, 'on': {'e': 'counters_added_source', 'counter': '+1/+1'},
                    'effects': [self.effect(args[0])], 'optional': len(args) == 2 and args[1].strip() == 'true'}
        # the predefined token abilities
        if cls == 'FoodAbility' and not args:
            return {'class': 'SimpleActivatedAbility', 'java': cls, 'effects': [{'class': 'GainLifeEffect', 'amount': 3}],
                    'cost': [{'class': 'ManaCost', 'cost': '{2}'}, {'class': 'TapSourceCost'},
                             {'class': 'SacrificeSourceCost'}]}
        if cls == 'ClueAbility' and args in (['true'], ['false']):
            return {'class': 'SimpleActivatedAbility', 'java': cls,
                    'effects': [{'class': 'DrawCardSourceControllerEffect', 'amount': 1}],
                    'cost': [{'class': 'ManaCost', 'cost': '{2}'}, {'class': 'SacrificeSourceCost'}]}
        if cls == 'TreasureAbility' and args in (['true'], ['false']):
            return {'class': 'AnyColorManaAbility', 'java': cls, 'cost': [{'class': 'SacrificeSourceCost'}]}
        # crew N: tap creatures with total power N: this becomes an artifact creature until end of turn
        if cls == 'CrewAbility' and len(args) == 1 and re.fullmatch(r'\d+', args[0].strip()):
            return {'class': 'SimpleActivatedAbility', 'java': cls,
                    'effects': [{'class': 'AddCardTypeSourceEffect', 'types': ['ARTIFACT', 'CREATURE'],
                                 'duration': 'EndOfTurn'}],
                    'cost': [{'class': 'CrewCost', 'value': int(args[0])}]}
        if cls == 'AnyColorManaAbility' and not args:
            return {'class': cls, 'java': cls}
        if cls == 'AnyColorManaAbility' and [a.strip() for a in args] == ['new TapSourceCost()']:
            return {'class': cls, 'java': cls}
        # "{T}: Add {G} for each …"
        if cls == 'DynamicManaAbility' and len(args) == 3 and re.fullmatch(r'"[^"]*"', args[2].strip()):
            args = args[:2]
        if cls == 'DynamicManaAbility' and len(args) == 2 \
                and (n := re.fullmatch(r'Mana\.(White|Blue|Black|Red|Green)Mana\(1\)', args[0].strip())):
            return {'class': cls, 'java': cls, 'color': COLOR_LETTERS[n.group(1)], 'amount': self.amount(args[1])}
        if cls == 'SimpleManaAbility' and len(args) == 3 and args[0].strip() == 'Zone.BATTLEFIELD' \
                and args[2].strip() == 'new TapSourceCost()':
            if (n := re.fullmatch(r'new AddManaOfAnyColorEffect\((\d+)\)', args[1].strip())):
                return {'class': 'AnyColorManaAbility', 'java': cls, 'amount': int(n.group(1))}
            if (n := re.fullmatch(r'Mana\.ColorlessMana\((\d+)\)', args[1].strip())):
                return {'class': 'ColorlessManaAbility', 'java': cls, 'amount': int(n.group(1))}
        # Elfhame Druid: "{T}: Add {G}{G}. Spend this mana only to cast kicked spells."
        if cls == 'ConditionalColoredManaAbility' and [a.strip() for a in args] == \
                ['new TapSourceCost()', 'Mana.GreenMana(2)', 'new ElfhameDruidManaBuilder()']:
            return {'class': 'ConditionalManaAbility', 'java': cls, 'color': 'G', 'amount': 2, 'filter': [{'p': 'kicked_spell'}]}
        # "{T}: Add N mana of any one color"
        if cls == 'SimpleManaAbility' and len(args) == 2 and args[1].strip() == 'new TapSourceCost()' \
                and (n := re.fullmatch(r'new AddManaOfAnyColorEffect\((\d+)\)', args[0].strip())):
            return {'class': 'AnyColorManaAbility', 'java': cls, 'amount': int(n.group(1))}
        # Cavern of Souls: any color, for a creature spell of the chosen type, which then can't be countered
        if cls == 'ConditionalAnyColorManaAbility' and [a.strip() for a in args] == \
                ['new TapSourceCost()', '1', 'new CavernOfSoulsManaBuilder()', 'true']:
            return {'class': 'ConditionalManaAbility', 'java': cls, 'filter': [{'p': 'type', 't': 'CREATURE'}],
                    'chosen_type': True}
        # "Add one mana of any color. Spend this mana only to cast an instant or sorcery spell."
        if cls == 'ConditionalAnyColorManaAbility' and [a.strip() for a in args] == ['1', 'new InstantOrSorcerySpellManaBuilder()']:
            return {'class': 'ConditionalManaAbility', 'java': cls, 'amount': 1,
                    'filter': [{'p': 'or', 'xs': [type_pred('INSTANT'), type_pred('SORCERY')]}]}
        # Castle Garenbrig: "{2}{G}{G}, {T}: Add six {G}. Spend this mana only to cast creature spells or activate
        # abilities of creatures."
        if cls == 'ConditionalColoredManaAbility' and len(args) == 3 \
                and args[2].strip() == 'new CastleGarenbrigManaBuilder()' \
                and (c := re.fullmatch(r'new ManaCostsImpl<>\("(\{[^"]*\})"\)', args[0].strip())) \
                and (n := re.fullmatch(r'Mana\.GreenMana\((\d+)\)', args[1].strip())):
            return {'class': 'ConditionalManaAbility', 'java': cls, 'color': 'G', 'amount': int(n.group(1)),
                    'filter': [{'p': 'type', 't': 'CREATURE'}], 'creature_abilities': True,
                    'cost': [{'class': 'ManaCost', 'cost': c.group(1)}]}
        # an Eldrazi Spawn or Scion: "Sacrifice this creature: Add {C}." (it doesn't tap)
        if cls == 'SimpleManaAbility' and len(args) == 3 and args[0].strip() == 'Zone.BATTLEFIELD' \
                and args[1].strip() == 'Mana.ColorlessMana(1)' and self.strip_setters(args[2].strip()) == 'new SacrificeSourceCost()':
            return {'class': 'ColorlessManaAbility', 'java': cls, 'cost': [{'class': 'SacrificeSourceCost'}], 'no_tap': True}
        # Arena of Glory: "{R}, {T}, Exert: Add {R}{R}" (its other costs and effect added after)
        if cls == 'SimpleManaAbility' and len(args) == 2 \
                and (n := re.fullmatch(r'new BasicManaEffect\(Mana\.(White|Blue|Black|Red|Green)Mana\((\d+)\)\)', args[0].strip())) \
                and (c := re.fullmatch(r'new ManaCostsImpl<>\("(\{[^"]*\})"\)', args[1].strip())):
            return {'class': 'ColoredManaAbility', 'java': cls, 'color': COLOR_LETTERS[n.group(1)],
                    'amount': int(n.group(2)), 'no_tap': True, 'cost': [{'class': 'ManaCost', 'cost': c.group(1)}]}
        # Heritage Druid: "Tap three untapped Elves you control: Add {G}{G}{G}." (it doesn't tap itself)
        if cls == 'SimpleManaAbility' and len(args) == 3 and args[0].strip() == 'Zone.BATTLEFIELD' \
                and re.fullmatch(r'new HeritageDruidManaEffect\(\w+\)', args[1].strip()) \
                and args[2].strip().startswith('new TapTargetCost('):
            return {'class': 'ColoredManaAbility', 'java': cls, 'color': 'G', 'amount': 3, 'no_tap': True,
                    'cost': [self.cost(args[2])]}
        # "{T}: Add {R}. Spend this mana only to cast a creature spell."
        if cls in ('ConditionalColoredManaAbility', 'ConditionalAnyColorManaAbility') and len(args) == 2 \
                and (b := re.fullmatch(r'new ConditionalSpellManaBuilder\((.*)\)', args[1].strip(), re.S)):
            if cls == 'ConditionalColoredManaAbility':
                n = re.fullmatch(r'Mana\.(White|Blue|Black|Red|Green)Mana\(1\)', args[0].strip())
                if n:
                    return {'class': 'ConditionalManaAbility', 'java': cls, 'color': COLOR_LETTERS[n.group(1)],
                            'filter': self.filter(b.group(1))}
            elif re.fullmatch(r'\d+', args[0].strip()):
                return {'class': 'ConditionalManaAbility', 'java': cls, 'amount': int(args[0]),
                        'filter': self.filter(b.group(1))}
        # "{T}: Add {R}. Activate only if …"
        if cls == 'ActivateIfConditionManaAbility' and len(args) == 4 and args[0] == 'Zone.BATTLEFIELD' \
                and args[2] == 'new TapSourceCost()':
            n = re.fullmatch(r'new BasicManaEffect\(Mana\.(White|Blue|Black|Red|Green)Mana\(1\)\)', args[1])
            if n:
                return {'class': cls, 'java': cls, 'color': COLOR_LETTERS[n.group(1)],
                        'condition': self.condition(args[3])}
            if args[1] == 'new AddManaOfAnyColorEffect()':
                return {'class': 'ActivateIfConditionAnyColorManaAbility', 'java': cls, 'cost': [],
                        'condition': self.condition(args[3])}
        # enters with counters: a replacement effect, as `EntersBattlefieldAbility` with `AddCountersSourceEffect`
        if cls == 'EntersBattlefieldWithCountersAbility' and len(args) in (1, 2):
            return {'class': 'EntersBattlefieldAbility', 'java': cls,
                    'effects': [{'class': 'AddCountersSourceEffect', 'counter': self.value('counter', args[0])}]}
        # `+N: …`: sorcery speed, paid with loyalty counters
        if cls == 'LoyaltyAbility' and len(args) == 2:
            n = re.fullmatch(r'[+-]?\d+', args[1].strip())
            if not n:
                raise Unsupported('loyalty cost ' + args[1])
            return {'class': cls, 'java': cls, 'effects': [self.effect(args[0])],
                    'cost': [{'class': 'PayLoyaltyCost', 'amount': int(args[1].strip())}]}
        if cls == 'GainLifeControllerTriggeredAbility' and len(args) == 5 and args[0].strip() in ('Zone.BATTLEFIELD', 'Zone.COMMAND'):
            args = args[1:]
        # an emblem's cast trigger (the command zone is where emblems are)
        if cls == 'SpellCastControllerTriggeredAbility' and len(args) == 5 and args[0].strip() == 'Zone.COMMAND':
            args = args[1:]
        # the spell a cast trigger points at only gives Shark Typhoon its X (the engine's X for cast triggers)
        if cls == 'SpellCastControllerTriggeredAbility' and len(args) == 4 \
                and args[0].strip() == 'new SharkTyphoonCastEffect()' and args[3].strip() == 'SetTargetPointer.SPELL':
            args = args[:3] + ['SetTargetPointer.NONE']
        # "when you do, …": its targets are added after (`DoWhenCostPaid`)
        if cls == 'ReflexiveTriggeredAbility' and len(args) == 2 and args[1].strip() == 'false':
            return {'class': 'Reflexive', 'effects': [self.effect(args[0])], 'targets': []}
        if cls == 'AtTheBeginOfNextUpkeepDelayedTriggeredAbility' and len(args) == 2 \
                and args[1].strip() == 'Duration.OneUse' and args[0].strip() == 'new DrawCardSourceControllerEffect(1)':
            return {'class': 'NextUpkeepDelayed', 'effects': [self.effect(args[0])],
                    'rule': "Draw a card at the beginning of the next turn's upkeep."}
        if cls == 'AtTheBeginOfNextEndStepDelayedTriggeredAbility' and len(args) == 1 \
                and args[0].strip() == 'new UntapLandsEffect(2)':
            return {'class': 'NextEndStepDelayed', 'effects': [self.effect(args[0])],
                    'rule': 'At the beginning of the next end step, untap up to two lands.'}
        t = self.more_triggers(cls, [x.strip() for x in args])
        if t is not None:
            return t
        if cls not in ABILITIES:
            raise Unsupported('ability ' + cls)
        a = self.build(cls, args, ABILITIES[cls], 'ability')
        if 'effect' in a:
            a['effects'] = [a.pop('effect')]
        return triggered(a)

    def creature_token(self, expr):
        """`new CreatureToken(p, t, "…").withColor("R").withSubType(SubType.GOBLIN).withAbility(…)…`"""
        expr = expr.strip()
        chain = []
        while True:
            m = re.fullmatch(r'(.*\))\s*\.(with\w+)\((.*)\)', expr, re.S)
            if not m or split_args(m.group(3)) != [m.group(3).strip()] and m.group(3).strip():
                break
            # the last call's argument must be the whole parenthesised tail
            inner = m.group(1)
            if inner.count('(') != inner.count(')'):
                break
            expr, chain = inner.strip(), [(m.group(2), m.group(3).strip())] + chain
        n = re.fullmatch(r'new CreatureToken\((-?\d+),(-?\d+)(?:,(".*?"))?((?:,SubType\.\w+)*)\)', expr, re.S)
        if not n:
            raise Unsupported('creature token ' + expr[:80])
        out = {'name': '', 'types': ['CREATURE'], 'colors': [], 'subtypes': [], 'power': int(n.group(1)),
               'toughness': int(n.group(2)), 'abilities': []}
        subtypes = subtype_names()
        # `CreatureToken(p, t, text, SubType... )`
        for st in re.findall(r'SubType\.(\w+)', n.group(4) or ''):
            if st not in subtypes:
                raise Unsupported('creature token subtype ' + st)
            out['subtypes'].append(subtypes[st])
        for call, arg in chain:
            if call == 'withColor' and re.fullmatch(r'"[WUBRG]+"', arg):
                out['colors'] += list(arg[1:-1])
            elif call == 'withSubType' and arg.startswith('SubType.') and arg[8:] in subtypes:
                out['subtypes'].append(subtypes[arg[8:]])
            elif call == 'withType' and arg.startswith('CardType.'):
                out['types'].append(arg[9:])
            elif call == 'withAllCreatureTypes' and arg == 'true':
                out['all_creature_types'] = True
            elif call == 'withAbility':
                a = self.ability(arg)
                if 'rule' not in a:
                    # the text a triggered ability was given, else a keyword's own
                    text = getattr(self, 'sources', {}).get(arg, arg)
                    r = re.search(r',\s*("(?:[^"\\]|\\.)*")\s*\)\s*$', self.strip_setters(text))
                    if r:
                        a['rule'] = json.loads(r.group(1))
                    elif a.get('class') == 'WardAbility' and a['effects'][0]['cost'][0]['class'] == 'ManaCost' \
                            and len(split_args(arg[len('new WardAbility('):-1])) == 1:
                        # `WardAbility.getRule` with its hint
                        c = a['effects'][0]['cost'][0]['cost']
                        a['rule'] = ('ward ' + c + ' <i>(Whenever this creature becomes the target of a spell or ability'
                                     ' an opponent controls, counter it unless that player pays ' + c + '.)</i>')
                    elif a.get('class') not in KEYWORDS:
                        raise Unsupported('creature token ability without a text ' + arg[:60])
                a.pop('java', None)
                out['abilities'].append(a)
            else:
                raise Unsupported('creature token ' + call + ' ' + arg)
        return out

    def more_triggers(self, cls, args):
        """Trigger classes whose overloads differ by argument type, read by shape; None if `cls` isn't one"""
        def flag(x):
            if x not in ('true', 'false'):
                raise Unsupported(cls + ' argument ' + x)
            return x == 'true'

        def out(on, effect, optional=False, pointer=False):
            return {'class': 'TriggeredAbility', 'java': cls, 'on': on, 'effects': [self.effect(effect)],
                    'optional': optional, 'set_target_pointer': pointer}

        n = len(args)
        # "when this dies, return it to its owner's hand at the beginning of the next end step"
        if cls == 'DiesSourceTriggeredAbility' and n == 3 and args[1] == 'false' and args[2] == 'SetTargetPointer.CARD' \
                and re.fullmatch(r'new CreateDelayedTriggeredAbilityEffect\(new AtTheBeginOfNextEndStepDelayedTriggeredAbility\('
                                 r'new ReturnToHandTargetEffect\(\)\)\)(\.setText\(.*\))?', args[0], re.S):
            return self.ability('new DiesSourceTriggeredAbility(new ReturnSourceToHandNextEndStepEffect(),false)')
        # "when this enters and at the beginning of your upkeep"
        if cls == 'OrTriggeredAbility' and n == 4 and args[0].strip() == 'Zone.BATTLEFIELD' \
                and args[2].strip() == 'new EntersBattlefieldTriggeredAbility(null)' \
                and args[3].strip() == 'new BeginningOfUpkeepTriggeredAbility(null)':
            return out({'e': 'enters_or_upkeep'}, args[1])
        # Voice of Resurgence: "whenever an opponent casts a spell during your turn or when this dies"
        if cls == 'OrTriggeredAbility' and n == 4 and args[0].strip() == 'Zone.BATTLEFIELD' \
                and canonical(args[2]) == 'new SpellCastOpponentTriggeredAbility(null,StaticFilters.FILTER_SPELL_A,false).withTriggerCondition(MyTurnCondition.instance)' \
                and canonical(args[3]) == 'new DiesSourceTriggeredAbility(null,false)':
            return out({'e': 'opponent_casts_on_your_turn_or_dies'}, args[1])
        if cls == 'DiesCreatureTriggeredAbility':
            if n >= 3 and args[2] not in ('true', 'false'):
                if n > 3 and flag(args[3]):
                    raise Unsupported(cls + ' with a target pointer')
                return out({'e': 'dies_all', 'filter': self.filter(args[2]), 'include_self': True}, args[0],
                           flag(args[1]))
            if n in (2, 3, 4):
                another = n >= 3 and flag(args[2])
                if n == 4 and flag(args[3]):
                    raise Unsupported(cls + ' with a target pointer')
                return out({'e': 'dies_all', 'filter': [CREATURE] + ([{'p': 'another'}] if another else []),
                            'include_self': not another}, args[0], flag(args[1]))
            raise Unsupported(cls + ' overload')
        # whenever a permanent of the filter deals combat damage to a player
        if cls == 'DealsDamageToAPlayerAllTriggeredAbility' and n == 5 and args[3] == 'SetTargetPointer.NONE' \
                and args[4] == 'true':
            return out({'e': 'combat_damage_to_player_all', 'filter': self.filter(args[1])}, args[0], flag(args[2]))
        if cls == 'AttacksAloneSourceTriggeredAbility' and n == 1:
            return out({'e': 'attacks_alone'}, args[0])
        # "at end of combat" (every combat)
        if cls == 'EndOfCombatTriggeredAbility' and n == 2 and args[1] in ('true', 'false'):
            return out({'e': 'step_begins', 'step': 'endcombat', 'who': 'any'}, args[0], flag(args[1]))
        # "when enchanted artifact is put into a graveyard, return that card to its owner's hand"
        if cls == 'DiesAttachedTriggeredAbility' and n == 4 and args[0] == 'new ReturnToHandAttachedEffect()' \
                and args[2:] == ['false', 'false']:
            return out({'e': 'dies_attached'}, 'new ReturnToHandTargetEffect()', False)
        # whenever a permanent of the filter leaves the battlefield
        if cls == 'LeavesBattlefieldAllTriggeredAbility' and n == 3 and args[2] in ('true', 'false'):
            return out({'e': 'leaves_all', 'filter': self.filter(args[1])}, args[0], flag(args[2]))
        if cls == 'AttacksWithCreaturesTriggeredAbility':
            if n == 2:
                return out({'e': 'attacks_with', 'min': int(args[1]), 'filter': [CREATURE]}, args[0])
            if n == 3:
                return out({'e': 'attacks_with', 'min': int(args[1]), 'filter': self.filter(args[2])}, args[0])
            raise Unsupported(cls + ' overload')
        if cls == 'AttacksCreatureYouControlTriggeredAbility':
            filt, pointer = [CREATURE, YOU], False
            if n == 3 and args[2] in ('true', 'false'):
                pointer = flag(args[2])
            elif n >= 3:
                filt = self.filter(args[2])
                pointer = n == 4 and flag(args[3])
            return out({'e': 'attacks_all', 'filter': filt}, args[0], n >= 2 and flag(args[1]), pointer)
        if cls == 'SacrificePermanentTriggeredAbility':
            if n == 6 and args[0] == 'Zone.BATTLEFIELD' and args[4] in ('SetTargetPointer.NONE', 'SetTargetPointer.PLAYER'):
                return out({'e': 'sacrificed', 'filter': self.filter(args[2]),
                            'who': self.value('target_controller', args[3])}, args[1], flag(args[5]),
                           args[4] == 'SetTargetPointer.PLAYER')
            if n in (2, 3):
                return out({'e': 'sacrificed', 'filter': self.filter(args[1]),
                            'who': self.value('target_controller', args[2]) if n == 3 else 'you'}, args[0])
            raise Unsupported(cls + ' overload')
        if cls == 'LimitedTimesPerTurnActivatedAbility' and n in (3, 4, 5) and args[0] == 'Zone.BATTLEFIELD' \
                and (n == 3 or re.fullmatch(r'\d+', args[3])):
            a = {'class': cls, 'java': cls, 'effects': [self.effect(args[1])], 'cost': [self.cost(args[2])],
                 'limit': int(args[3]) if n > 3 else 1}
            if n == 5:
                a['condition'] = self.condition(args[4])
            return a
        # "whenever you tap a creature for mana, add an additional {G}"
        if cls == 'TapForManaAllTriggeredManaAbility' and n == 3 and args[2] == 'SetTargetPointer.NONE' \
                and (m := re.fullmatch(r'new BasicManaEffect\(Mana\.(White|Blue|Black|Red|Green)Mana\(1\)\)',
                                       self.strip_setters(args[0]))):
            return {'class': cls, 'java': cls, 'filter': self.filter(args[1]),
                    'color': {'White': 'W', 'Blue': 'U', 'Black': 'B', 'Red': 'R', 'Green': 'G'}[m.group(1)]}
        # gift (an instant's or sorcery's: the gift is given as it resolves, before its effects)
        if cls == 'GiftAbility' and n == 2 and args[0] == 'this' and args[1] in GIFTS:
            name, tok = GIFTS[args[1]]
            out = {'class': cls, 'java': cls, 'name': name}
            if tok:
                out['token'] = token(tok)
            return out
        # training: "whenever this attacks with another creature with greater power, put a +1/+1 counter on this"
        if cls == 'TrainingAbility' and n == 0:
            return {'class': 'TriggeredAbility', 'java': cls, 'on': {'e': 'training'}, 'optional': False,
                    'effects': [{'class': 'AddCountersSourceEffect', 'counter': {'name': '+1/+1', 'amount': 1}}]}
        # firebending N: "whenever this attacks, add N {R}; it lasts until end of combat"
        if cls == 'FirebendingAbility' and n == 1 and re.fullmatch(r'\d+', args[0]):
            return {'class': 'TriggeredAbility', 'java': cls, 'on': {'e': 'attacks'}, 'optional': False,
                    'effects': [{'class': 'FirebendingEffect', 'amount': int(args[0])}]}
        if cls == 'AlternativeCostSourceAbility' and n == 3 and args[2].strip().startswith('"'):
            return {'class': cls, 'java': cls, 'costs': [self.cost(args[0])], 'condition': self.condition(args[1])}
        if cls == 'GoblinGuideTriggeredAbility' and args == ['new GoblinGuideEffect()', 'false']:
            return {'class': 'TriggeredAbility', 'java': cls, 'on': {'e': 'attacks'}, 'optional': False,
                    'effects': [{'class': 'GoblinGuideEffect'}]}
        if cls in ('SpreeAbility', 'TieredAbility') and args == ['this']:
            return {'class': cls, 'java': cls}
        if cls == 'OffspringAbility' and n == 1 and re.fullmatch(r'"(\{[^"]*\})+"', args[0]):
            return {'class': cls, 'java': cls, 'cost': args[0][1:-1]}
        if cls == 'BlitzAbility' and n == 2 and args[0] == 'this' and re.fullmatch(r'"(\{[^"]*\})+"', args[1]):
            return {'class': cls, 'java': cls, 'mana': args[1][1:-1], 'cost': []}
        if cls == 'WardAbility' and n in (1, 2) and (n == 1 or args[1] in ('true', 'false')):
            return {'class': 'WardAbility', 'java': cls, 'hint': n == 1 or args[1] == 'true',
                    'effects': [{'class': 'CounterUnlessPaysEffect', 'cost': [self.cost(args[0])]}]}
        if cls == 'ChannelAbility' and n == 2 and re.fullmatch(r'"(\{[^"]*\})"', args[0]):
            return {'class': 'ChannelAbility', 'java': cls, 'cost': [{'class': 'ManaCost', 'cost': args[0][1:-1]},
                                                                     {'class': 'DiscardSourceCost'}],
                    'effects': [self.effect(args[1])]}
        # the fetch lands: {T}, pay 1 life (unless withDamage is false), sacrifice it: a land card of either subtype
        if cls == 'FetchLandActivatedAbility' and n in (2, 3):
            subs = args[-2:]
            if not all(re.fullmatch(r'SubType\.[A-Z]+', s) for s in subs) or (n == 3 and args[0] not in ('true', 'false')):
                raise Unsupported('FetchLandActivatedAbility ' + ','.join(args))
            cost = [{'class': 'TapSourceCost'}]
            if n == 2 or args[0] == 'true':
                cost.append({'class': 'PayLifeCost', 'amount': 1})
            cost.append({'class': 'SacrificeSourceCost'})
            names = [self.value('subtype', s) for s in subs]
            return {'class': 'SimpleActivatedAbility', 'java': cls, 'cost': cost,
                    'effects': [{'class': 'SearchLibraryEffect', 'filter': [{'p': 'or', 'xs': [{'p': 'subtype', 's': x} for x in names]}],
                                 'min': 1, 'max': 1, 'to': 'BATTLEFIELD', 'optional': False}]}
        if cls == 'CyclingAbility' and n == 1 and (m := re.fullmatch(r'new GenericManaCost\((\d+)\)', args[0])):
            return {'class': 'CyclingAbility', 'java': cls, 'cost': [{'class': 'ManaCost', 'cost': '{' + m.group(1) + '}'},
                                                                     {'class': 'DiscardSourceCost'}],
                    'effects': [{'class': 'DrawCardSourceControllerEffect', 'amount': 1}]}
        if cls == 'CyclingAbility' and n == 1 and (m := re.fullmatch(r'new ManaCostsImpl<>\("(\{[^"]*\})"\)', args[0])):
            return {'class': 'CyclingAbility', 'java': cls, 'cost': [{'class': 'ManaCost', 'cost': m.group(1)},
                                                                     {'class': 'DiscardSourceCost'}],
                    'effects': [{'class': 'DrawCardSourceControllerEffect', 'amount': 1}]}
        if cls == 'DealsCombatDamageToAPlayerOrPlaneswalkerTriggeredAbility' and n == 2:
            return out({'e': 'combat_damage_to_player_or_planeswalker'}, args[0], flag(args[1]))
        if cls == 'FlurryAbility' and n == 1:
            return out({'e': 'cast_second_spell', 'who': 'you'}, args[0])
        if cls == 'CastSecondSpellTriggeredAbility' and n == 2:
            return out({'e': 'cast_second_spell', 'who': self.value('target_controller', args[1])}, args[0])
        if cls == 'BecomesExertSourceTriggeredAbility' and n == 1:
            return out({'e': 'exerted'}, args[0])
        if cls == 'CycleTriggeredAbility' and n == 1:
            return out({'e': 'cycled'}, args[0])
        if cls == 'CommittedCrimeTriggeredAbility' and n == 1:
            return out({'e': 'committed_crime'}, args[0])
        if cls == 'EntersBattlefieldOrLeavesSourceTriggeredAbility' and n == 2:
            return out({'e': 'enters_or_leaves'}, args[0], flag(args[1]))
        # "whenever one or more artifacts you control leave the battlefield during your turn": once each turn
        if cls == 'OniCultAnvilTriggeredAbility' and n == 0:
            a = self.ability('new LeavesBattlefieldAllTriggeredAbility(new CreateTokenEffect(new ConstructToken()),'
                             'StaticFilters.FILTER_CONTROLLED_PERMANENT_ARTIFACTS,false)')
            a['java'] = cls
            a['limit_per_turn'] = 1
            a['trigger_condition'] = {'c': 'my_turn'}
            return a
        if cls == 'ForageTriggeredAbility' and n == 1:
            return out({'e': 'forage'}, args[0])
        if cls == 'ExpendTriggeredAbility' and n == 2 and (m := re.fullmatch(r'ExpendTriggeredAbility\.Expend\.(FOUR|SIX|EIGHT)', args[1])):
            return out({'e': 'expend', 'n': {'FOUR': 4, 'SIX': 6, 'EIGHT': 8}[m.group(1)]}, args[0])
        # once for each batch: one sacrifice at a time in these decks
        if cls == 'SacrificeOneOrMorePermanentsTriggeredAbility' and n == 2:
            return out({'e': 'sacrificed_batch', 'filter': self.filter(args[1]), 'who': 'you'}, args[0])
        if cls == 'ValiantTriggeredAbility' and n == 1:
            return out({'e': 'valiant'}, args[0])
        if cls == 'BecomesTargetSourceTriggeredAbility' and n == 4 and args[2] in ('SetTargetPointer.PLAYER', 'SetTargetPointer.NONE'):
            return out({'e': 'becomes_target_of_spell', 'filter': self.filter(args[1])}, args[0], flag(args[3]),
                       args[2] == 'SetTargetPointer.PLAYER')
        # "when this dies, put it on the bottom of its owner's library"
        if cls == 'DiesSourceTriggeredAbility' and n == 3 and args[2] == 'SetTargetPointer.CARD' \
                and (m := re.fullmatch(r'new PutOnLibraryTargetEffect\((true|false)\)', self.strip_setters(args[0]))):
            return {'class': 'TriggeredAbility', 'java': cls, 'on': {'e': 'dies'},
                    'effects': [{'class': 'PutOnLibrarySourceEffect', 'top': m.group(1) == 'true'}],
                    'optional': flag(args[1]), 'set_target_pointer': False}
        if cls == 'HeroicAbility' and n in (1, 2, 3):
            return out({'e': 'spell_cast_targeting_this'}, args[0], n == 2 and flag(args[1]))
        # (the third argument: the enrage ability word)
        if cls == 'DealtDamageToSourceTriggeredAbility' and (n == 2 or (n == 3 and args[2] in ('true', 'false'))):
            return out({'e': 'dealt_damage'}, args[0], flag(args[1]))
        # whenever you draw your second card each turn
        if cls == 'DrawNthCardTriggeredAbility' and n == 1:
            return out({'e': 'draws_nth', 'n': 2}, args[0])
        if cls == 'DrawCardControllerTriggeredAbility' and n == 2:
            return out({'e': 'draws', 'who': 'you'}, args[0], flag(args[1]))
        if cls == 'DrawCardControllerTriggeredAbility' and n == 3 and args[0] == 'Zone.COMMAND':
            return out({'e': 'draws', 'who': 'you'}, args[1], flag(args[2]))
        if cls == 'DrawCardOpponentTriggeredAbility' and n == 3:
            return out({'e': 'draws', 'who': 'opponent'}, args[0], flag(args[1]), flag(args[2]))
        if cls == 'DealsDamageToOpponentTriggeredAbility' and 1 <= n <= 4:
            return out({'e': 'deals_damage_to_opponent', 'only_combat': n >= 3 and flag(args[2])}, args[0],
                       n >= 2 and flag(args[1]), n == 4 and flag(args[3]))
        if cls == 'LeavesBattlefieldTriggeredAbility' and n in (1, 2):
            return out({'e': 'leaves_battlefield'}, args[0], n == 2 and flag(args[1]))
        if cls == 'SpellCastAllTriggeredAbility':
            if n == 2:
                return out({'e': 'spell_cast', 'filter': [], 'who': 'any'}, args[0], flag(args[1]))
            if n == 3 or (n == 4 and args[3] == 'SetTargetPointer.NONE'):
                return out({'e': 'spell_cast', 'filter': self.filter(args[1]), 'who': 'any'}, args[0], flag(args[2]))
            if n == 4 and args[3] == 'SetTargetPointer.SPELL':
                return out({'e': 'spell_cast', 'filter': self.filter(args[1]), 'who': 'any'}, args[0], flag(args[2]), True)
            if n == 4 and args[3] == 'SetTargetPointer.PLAYER':
                a = out({'e': 'spell_cast', 'filter': self.filter(args[1]), 'who': 'any'}, args[0], flag(args[2]))
                a['set_target_pointer_player'] = True
                return a
            raise Unsupported(cls + ' overload')
        # whenever `who` loses life; "that much" is the trigger's X
        if cls == 'LoseLifeTriggeredAbility' and n == 4:
            return out({'e': 'loses_life', 'who': self.value('target_controller', args[1])}, args[0], flag(args[2]),
                       flag(args[3]))
        # whenever a permanent of the filter enters under an opponent's control
        if cls == 'EntersBattlefieldOpponentTriggeredAbility' and n == 3:
            return out({'e': 'enters_all', 'filter': self.filter(args[1]) + [{'p': 'controller', 'who': 'opponent'}]},
                       args[0], flag(args[2]))
        if cls == 'EntersBattlefieldThisOrAnotherTriggeredAbility' and n == 4:
            filt = self.filter(args[1]) + ([YOU] if flag(args[3]) else [])
            return out({'e': 'enters_all', 'filter': filt, 'include_self': True}, args[0], flag(args[2]))
        return None

    def assign(self, var, expr):
        """`Effect effect = new X(…)`: what kind of thing the right side is decides how it's read"""
        self.sources = getattr(self, 'sources', {})
        self.sources[var] = expr
        if re.search(r'\.withInterveningIf\(\w+(?:\.instance|\.ONCE)?\)\s*$', self.strip_setters(expr)):
            self.vars[var] = self.ability(expr)
            return
        obj = self.new_expr(expr)
        if isinstance(obj, dict):
            self.vars[var] = obj
            return
        cls = obj[0]
        # a `DynamicValue` (`AuraAttachedCount count = new AuraAttachedCount(2)`): read where it's used
        if re.search(r'(Count|Value)$', cls):
            self.values[var] = expr
            return
        if cls == 'Mode':
            args = self.new_expr(expr)[1]
            self.vars[var] = {'class': 'Mode', 'effects': [self.effect(a) for a in args], 'targets': []}
        elif cls in EFFECTS or cls.endswith('Effect'):
            self.vars[var] = self.effect(expr)
        elif cls.endswith('Cost'):
            self.vars[var] = self.cost(expr)
        elif cls == 'TargetCardInLibrary':
            self.vars[var] = {'library_target': self.library_target(expr)}
        elif cls in TARGET_CLASSES or cls in PLAYER_TARGETS:
            self.vars[var] = self.target(expr)
        else:
            self.vars[var] = self.ability(expr)


# spells whose effects work across their targets in ways the statements can't say: their spell, by card name
SPELL_OVERRIDES = {
    'Dark Salvation': lambda: {
        'effects': [{'class': 'DarkSalvationEffect', 'token': token('ZombieToken')}],
        'targets': [{'class': 'TargetPlayer'}, {'class': 'TargetPermanent', 'filter': [CREATURE], 'min': 0, 'max': 1}]},
}


def translate(card_name, subtypes, facts):
    if card_name not in facts:
        raise Unsupported('no card facts')
    java_abilities = facts[card_name]
    if card_name in BASIC_LANDS:
        return {'name': card_name, 'types': ['LAND'], 'supertypes': ['BASIC'], 'subtypes': [card_name],
                'colors': [], 'abilities': [{'class': 'PlayLandAbility'}, {'class': BASIC_LANDS[card_name]}]}
    cls = class_name(card_name)
    path = os.path.join(SETS, cls[0].lower(), cls + '.java')
    if not os.path.exists(path) and ' // ' in card_name:
        # a split card (a Room) is named after both halves
        cls = ''.join(class_name(n) for n in card_name.split(' // '))
        path = os.path.join(SETS, cls[0].lower(), cls + '.java')
    if not os.path.exists(path):
        raise Unsupported('no card class ' + cls)
    source = open(path).read()
    if 'extends ModalDoubleFacedCard' in re.search(r'public final class \w+ extends \w+', source).group(0):
        return modal_double_faced(card_name, subtypes, facts, cls, source)
    if 'extends TransformingDoubleFacedCard' in re.search(r'public final class \w+ extends \w+', source).group(0):
        return transforming_double_faced(card_name, subtypes, facts, cls, source)
    if 'extends RoomCard' in re.search(r'public final class \w+ extends \w+', source).group(0):
        return room_card(card_name, subtypes, facts, cls, source)
    if 'extends SplitCard' in re.search(r'public final class \w+ extends \w+', source).group(0) \
            and 'SpellAbilityType.SPLIT_AFTERMATH' in source:
        return aftermath_card(card_name, subtypes, facts, cls, source)
    if re.search(r'extends (?!CardImpl\b|AdventureCard\b|OmenCard\b|PrepareCard\b)\w+', re.search(r'public final class \w+ extends \w+', source).group(0)):
        raise Unsupported('card class is not a CardImpl')
    t = Translator(declared_filters(source, subtypes), card_conditions(source), subtypes, card_values(source))
    # filters the constructor declares for itself (`FilterX filter = new FilterX(…); filter.add(…);`)
    body = constructor_body(source, cls)
    local_filters = set()
    for f in re.finditer(r'\n\s*(?:final )?(Filter\w+) (\w+)\s*=\s*new (Filter\w+)\((.*?)\);', body, re.S):
        adds = [canonical(p) for p in re.findall(r'\b' + f.group(2) + r'\.add\((.*?)\);', body, re.S)]
        try:
            t.filters[f.group(2)] = filter_object(f.group(3), split_args(canonical(f.group(4))), adds, subtypes)
            local_filters.add(f.group(2))
        except Unsupported:
            pass
    card = {'name': card_name}
    added, spell, modes = [], {'effects': [], 'targets': []}, []
    # an adventurer card's adventure: its spell is `getSpellCard().getSpellAbility()`
    adventure, aspell, amodes = None, {'effects': [], 'targets': []}, []
    errors = []
    for st in statements(body):
        def one(st):
            nonlocal adventure
            if (n := re.fullmatch(r'(?:final )?Filter\w+ (\w+)=new Filter.*', st) or re.fullmatch(r'(\w+)\.add\(.*\)', st)) \
                    and n.group(1) in local_filters:
                return
            m = re.fullmatch(r'super\(ownerId,setInfo,new CardType\[\]\{([^}]*)\},new CardType\[\]\{([^}]*)\},'
                             r'"(.*?)","(.*?)","(.*?)"\)', st)
            if m:
                card['types'] = [x.strip().split('.')[-1] for x in m.group(1).split(',') if x.strip()]
                card['cost'] = m.group(3)
                adventure = {'name': m.group(4), 'cost': m.group(5),
                             'types': [x.strip().split('.')[-1] for x in m.group(2).split(',') if x.strip()]}
                return
            # a preparation card (`PrepareCard`): its prepare spell, kept as the card's `prepare`
            m = re.fullmatch(r'super\(ownerId,setInfo,new CardType\[\]\{([^}]*)\},"(.*?)","(.*?)",'
                             r'(?:new CardType\[\]\{([^}]*)\}|(CardType\.\w+)),"(.*?)"\)', st)
            if m:
                card['types'] = [x.strip().split('.')[-1] for x in m.group(1).split(',') if x.strip()]
                card['cost'] = m.group(2)
                adventure = {'name': m.group(3), 'cost': m.group(6), 'prepare': True,
                             'types': [x.strip().split('.')[-1] for x in (m.group(4) or m.group(5)).split(',') if x.strip()]}
                return
            if st == 'this.finalizeAdventure()':
                return
            if st == 'this.finalizeOmen()':
                adventure['omen'] = True
                return
            if card_name in SPELL_OVERRIDES and ('getSpellAbility()' in st or re.match(r'(Effect\s+)?effect\b', st)):
                return
            if st.startswith('this.getSpellCard().getSpellAbility()'):
                card_statement(t, st.replace('this.getSpellCard().getSpellAbility()', 'this.getSpellAbility()', 1),
                               {}, [], aspell, amodes, subtypes)
                return
            card_statement(t, st, card, added, spell, modes, subtypes)
        if ALL_ERRORS:
            try:
                one(st)
            except Unsupported as e:
                errors.append(str(e))
            continue
        one(st)
    if errors:
        raise Unsupported(' ;; '.join(errors))
    if 'types' not in card:
        raise Unsupported('no card types')
    if card_name in SPELL_OVERRIDES:
        spell.update(SPELL_OVERRIDES[card_name]())
    card['abilities'] = in_java_order(java_abilities, added)
    # a creature cast from suspend gains haste, which the engine doesn't do yet
    if 'CREATURE' in card.get('types', []) and any(a.get('class') == 'SuspendAbility' for a in card['abilities']):
        raise Unsupported('suspend on a creature')
    rule = next((f['rule'] for f in java_abilities if f['class'] == 'SpellAbility'), '')
    # spree: one or more modes, each with its own cost
    if any(a['class'] == 'SpreeAbility' for a in card['abilities']):
        if not modes or 'first_mode_cost' not in spell or not all('cost' in m for m in modes):
            raise Unsupported('spree without mode costs')
        spell['min_modes'], spell['max_modes'] = 1, len(modes) + 1
    elif any(a['class'] == 'TieredAbility' for a in card['abilities']):
        # tiered: choose one, with its cost
        if not modes or 'first_mode_cost' not in spell or not all('cost' in m for m in modes):
            raise Unsupported('tiered without mode costs')
    elif 'first_mode_cost' in spell or any('cost' in m for m in modes):
        raise Unsupported('mode costs without spree')
    if modes:
        # "choose one": the spell ability's own effects and targets are its first mode
        first = {'effects': spell['effects'], 'targets': spell['targets']}
        if 'first_mode_cost' in spell:
            first['cost'] = spell['first_mode_cost']
        card['spell'] = {'modes': [first] + modes, 'rule': rule}
        for k in ('min_modes', 'max_modes'):
            if k in spell:
                card['spell'][k] = spell[k]
    elif spell['effects'] or spell['targets'] or spell.get('costs'):
        card['spell'] = dict(spell, rule=rule)
    if adventure is not None:
        if amodes:
            raise Unsupported('modal adventure')
        arule = next((f['rule'] for f in java_abilities
                      if f['class'] in ('AdventureCardSpellAbility', 'OmenCardSpellAbility', 'PrepareCardSpellAbility')), '')
        adventure['spell'] = dict(aspell, rule=arule)
        if adventure.pop('prepare', False):
            card['prepare'] = adventure
        else:
            card['adventure'] = adventure
    elif 'AdventureCardSpellAbility' in [f['class'] for f in java_abilities]:
        raise Unsupported('adventure without its spell')
    return card


def aftermath_card(card_name, subtypes, facts, cls, source):
    """An aftermath split card (`SplitCard`, `SPLIT_AFTERMATH`): its halves (`room`, left then right) are each a spell,
    the left cast from hand and the right only from the graveyard, then exiled (`aftermath`); in other zones the card
    has both halves' types and mana cost"""
    halves = EXTRA.get(card_name, {}).get('halves')
    if not halves or len(halves) != 2:
        raise Unsupported('split card without its halves\' facts')
    t = Translator(declared_filters(source, subtypes), card_conditions(source), subtypes, card_values(source))
    card = {'name': card_name, 'aftermath': True, 'abilities': []}
    faces = [{'name': halves[0]['name']}, {'name': halves[1]['name']}]
    spells = [{'effects': [], 'targets': []}, {'effects': [], 'targets': []}]
    modes = [[], []]
    types = lambda g: [x.strip().split('.')[-1] for x in g.split(',') if x.strip()]
    for st in statements(constructor_body(source, cls)):
        m = re.fullmatch(r'super\(ownerId,setInfo,new CardType\[\]\{([^}]*)\},new CardType\[\]\{([^}]*)\},'
                         r'"(.*?)","(.*?)",SpellAbilityType\.SPLIT_AFTERMATH\)', st)
        if m:
            faces[0]['types'], faces[1]['types'] = types(m.group(1)), types(m.group(2))
            faces[0]['cost'], faces[1]['cost'] = m.group(3), m.group(4)
            continue
        if re.fullmatch(r'(?:this\.)?getRightHalfCard\(\)\.addAbility\(new AftermathAbility\(\)(?:\.setRuleAtTheTop\(true\))?\)', st):
            continue
        m = re.fullmatch(r'(?:this\.)?get(Left|Right)HalfCard\(\)\.(getSpellAbility\(\)\..*)', st)
        if m:
            k = m.group(1) == 'Right'
            card_statement(t, 'this.' + m.group(2), faces[k], [], spells[k], modes[k], subtypes)
            continue
        raise Unsupported('split card statement ' + st)
    if 'types' not in faces[0] or modes[0] or modes[1]:
        raise Unsupported('split card without its halves')
    for face, spell, half in zip(faces, spells, halves):
        face['abilities'] = [{'class': 'SpellAbility'}]
        rule = next((a['rule'] for a in half['abilities'] if a['class'] == 'SpellAbility'), '')
        face['spell'] = dict(spell, rule=rule)
    card['types'] = list(dict.fromkeys(faces[0]['types'] + faces[1]['types']))
    card['cost'] = faces[0]['cost'] + faces[1]['cost']
    card['room'] = faces
    return card


def room_card(card_name, subtypes, facts, cls, source):
    """A Room (`RoomCard`): an enchantment whose halves are each cast as a spell (its `room`, left then right) and
    unlock as doors on the battlefield; the card's own abilities are its spell, the door unlocked on entering and the
    Room's static ability"""
    java_abilities = facts[card_name]
    halves = EXTRA.get(card_name, {}).get('halves')
    if not halves or len(halves) != 2:
        raise Unsupported('room without its halves\' facts')
    t = Translator(declared_filters(source, subtypes), card_conditions(source), subtypes, card_values(source))
    card = {'name': card_name, 'types': ['ENCHANTMENT'], 'subtypes': ['Room']}
    added = [[], []]
    costs = None
    for st in statements(constructor_body(source, cls)):
        m = re.fullmatch(r'super\(ownerId,setInfo,"(.*?)","(.*?)"\)', st)
        if m:
            costs = m.group(1, 2)
            continue
        m = re.fullmatch(r'this\.get(Left|Right)HalfCard\(\)\.(addAbility\(.*\))', st)
        if m:
            if not ability_statement(t, 'this.' + m.group(2), added[m.group(1) == 'Right']):
                raise Unsupported('room statement ' + st)
            continue
        if not ability_statement(t, st, []):
            raise Unsupported('room statement ' + st)
    if costs is None or [f['class'] for f in java_abilities] != ['SpellAbility', 'EntersBattlefieldAbility', 'RoomAbility']:
        raise Unsupported('room without its costs')
    # the card's mana cost is both halves' (`SplitCard`)
    card['cost'] = costs[0] + costs[1]
    card['abilities'] = [{'class': 'SpellAbility'}, {'class': 'RoomEnterUnlockAbility'},
                         {'class': 'RoomAbility', 'rule': java_abilities[2]['rule']}]
    card['room'] = []
    for half, cost, side in zip(halves, costs, added):
        card['room'].append({'name': half['name'], 'cost': cost, 'types': ['ENCHANTMENT'], 'subtypes': ['Room'],
                             'abilities': in_java_order(half['abilities'], side)})
    return card


def transforming_double_faced(card_name, subtypes, facts, cls, source):
    """A transforming double-faced card (`TransformingDoubleFacedCard`): the front is the card, the back what it
    transforms into (`transform_back`); Java lists the front's abilities (its spell ability first), then the back's"""
    java_abilities = facts[card_name]
    t = Translator(declared_filters(source, subtypes), card_conditions(source), subtypes, card_values(source))
    front, back = {'name': card_name}, {}
    added = [[], []]
    types = lambda g: [x.strip().split('.')[-1] for x in g.split(',') if x.strip()]
    subs = lambda g: [subtypes[x.strip().split('.')[-1]] for x in g.split(',') if x.strip()]
    sup = r'(?:new SuperType\[\]\{([^}]*)\},)?'
    for st in statements(constructor_body(source, cls)):
        m = re.fullmatch(r'super\(ownerId,setInfo,' + sup + r'new CardType\[\]\{([^}]*)\},new SubType\[\]\{([^}]*)\},"(.*?)",'
                         r'"(.*?)",' + sup + r'new CardType\[\]\{([^}]*)\},new SubType\[\]\{([^}]*)\},"([WUBRG]*)"\)', st)
        if m:
            for face, (su, ty, sb) in ((front, m.group(1, 2, 3)), (back, m.group(6, 7, 8))):
                face['types'] = types(ty)
                face['subtypes'] = subs(sb)
                if su and types(su):
                    face['supertypes'] = types(su)
            front['cost'] = m.group(4)
            back['name'] = m.group(5)
            # a transformed permanent's mana value is its front's
            back['cost'] = m.group(4)
            back['colors'] = list(m.group(9))
            continue
        m = re.fullmatch(r'this\.getLeftHalfCard\(\)\.setStartingDefense\((\d+)\)', st)
        if m:
            front['defense'] = int(m.group(1))
            continue
        m = re.fullmatch(r'this\.get(Left|Right)HalfCard\(\)\.setPT\((-?\d+),(-?\d+)\)', st)
        if m:
            face = back if m.group(1) == 'Right' else front
            face['power'], face['toughness'] = int(m.group(2)), int(m.group(3))
            continue
        # a sorcery front (Esper Origins): its spell's effects
        m = re.fullmatch(r'this\.getLeftHalfCard\(\)\.getSpellAbility\(\)\.addEffect\((.*)\)', st)
        if m:
            front.setdefault('spell', {'effects': [], 'targets': [], 'rule': java_abilities[0].get('rule', '')})
            front['spell']['effects'].append(t.effect(m.group(1)))
            continue
        m = re.fullmatch(r'this\.get(Left|Right)HalfCard\(\)\.(addAbility\(.*\))', st)
        if m:
            # (a face given as the card: `new FlashbackAbility(this.getLeftHalfCard(), …)`)
            body = m.group(2).replace('this.getLeftHalfCard()', 'this').replace('this.getRightHalfCard()', 'this')
            if not ability_statement(t, 'this.' + body, added[m.group(1) == 'Right']):
                raise Unsupported('transforming double-faced card statement ' + st)
            continue
        # a watcher is the engine's own bookkeeping
        if re.fullmatch(r'\w+\.addWatcher\(new \w+Watcher\(\)\)', st):
            continue
        # a Saga front: its lore counter and chapters are the front's abilities
        saga = re.search(r'new SagaAbility\(this\.get(Left|Right)HalfCard\(\)\)|\.addChapterEffect\(this\.get(Left|Right)HalfCard\(\),', st)
        side = added[(saga.group(1) or saga.group(2)) == 'Right'] if saga else []
        if not ability_statement(t, st, side):
            raise Unsupported('transforming double-faced card statement ' + st)
    if 'types' not in front or not java_abilities or java_abilities[0]['class'] != 'SpellAbility':
        raise Unsupported('transforming double-faced card without its faces')
    split = 1 + len(added[0])
    front['abilities'] = in_java_order(java_abilities[:split], added[0])
    back['abilities'] = in_java_order(java_abilities[split:], added[1])
    front['transform_back'] = back
    return front


def modal_double_faced(card_name, subtypes, facts, cls, source):
    """A modal double-faced card whose faces are both lands (the pathways): each face is a card of its own, and the
    card's abilities are the front's, then the back's"""
    java_abilities = facts[card_name]
    t = Translator(declared_filters(source, subtypes), card_conditions(source), subtypes, card_values(source))
    faces = [{'name': card_name}, {}]
    added = [[], []]
    for st in statements(constructor_body(source, cls)):
        m = re.fullmatch(r'super\(ownerId,setInfo,new CardType\[\]\{([^}]*)\},new SubType\[\]\{\},"","(.*?)",'
                         r'new CardType\[\]\{([^}]*)\},new SubType\[\]\{\},""\)', st)
        if m:
            for face, types in ((faces[0], m.group(1)), (faces[1], m.group(3))):
                face['types'] = [x.strip().split('.')[-1] for x in types.split(',') if x.strip()]
                if face['types'] != ['LAND']:
                    raise Unsupported('modal double-faced card with a face that is not a land')
                face['cost'] = ''
            faces[1]['name'] = m.group(2)
            continue
        m = re.fullmatch(r'this\.get(Left|Right)HalfCard\(\)\.(addAbility\(.*\))', st)
        if not m or not ability_statement(t, 'this.' + m.group(2), added[m.group(1) == 'Right']):
            raise Unsupported('modal double-faced card statement ' + st)
    if 'types' not in faces[0]:
        raise Unsupported('modal double-faced card without its faces')
    split = [i for i, f in enumerate(java_abilities) if f['class'] == 'PlayLandAbility']
    if len(split) != 2:
        raise Unsupported('modal double-faced card abilities')
    faces[0]['abilities'] = in_java_order(java_abilities[:split[1]], added[0])
    faces[1]['abilities'] = in_java_order(java_abilities[split[1]:], added[1])
    faces[0]['back'] = faces[1]
    return faces[0]


def card_statement(t, st, card, added, spell, modes, subtypes):
    """One statement of a card's constructor"""
    st = re.sub(r'^(subtype|supertype|power|toughness)\b', r'this.\1', st)
    if True:
        m = re.fullmatch(r'super\(ownerId,setInfo,new CardType\[\]\{([^}]*)\},(".*?"|null)\)', st)
        if m:
            card['types'] = [x.strip().split('.')[-1] for x in m.group(1).split(',') if x.strip()]
            card['cost'] = '' if m.group(2) == 'null' else m.group(2)[1:-1]
            return
        m = re.fullmatch(r'this\.subtype\.add\((.*)\)', st)
        if m:
            for x in split_args(m.group(1)):
                key = x.split('.')[-1]
                if key not in subtypes:
                    raise Unsupported('subtype ' + x)
                card.setdefault('subtypes', []).append(subtypes[key])
            return
        # a cost's rule text (`cost.setText(…)`) is display
        m = re.fullmatch(r'(\w+)\.setText\(.*\)', st, re.S)
        if m and isinstance(t.vars.get(m.group(1)), dict) and t.vars[m.group(1)].get('class', '').endswith('Cost'):
            return
        m = re.fullmatch(r'this\.supertype\.add\((.*)\)', st)
        if m:
            card.setdefault('supertypes', []).extend(x.split('.')[-1] for x in split_args(m.group(1)))
            return
        m = re.fullmatch(r'this\.setStartingLoyalty\((\d+)\)', st)
        if m:
            card['loyalty'] = int(m.group(1))
            return
        # devoid: the card has no colors
        if re.fullmatch(r'this\.addAbility\(new DevoidAbility\(this\.color\)\)', st):
            card['colors'] = []
            added.append({'class': 'DevoidAbility'})
            return
        # the frame's colors are only the card's look
        if re.fullmatch(r'this\.frameColor=new ObjectColor\("[WUBRG]*"\)', st):
            return
        m = re.fullmatch(r'this\.(power|toughness)=new MageInt\((-?\d+)\)', st)
        if m:
            card[m.group(1)] = int(m.group(2))
            return
        if ability_statement(t, st, added):
            return
        st = re.sub(r'^getSpellAbility\(\)', 'this.getSpellAbility()', st)
        st = re.sub(r'^this\.getSpellAbility\(\)\.get(Effect|Target)s\(\)\.add\(', r'this.getSpellAbility().add\1(', st)
        # Ice Out: {1} less if it's bargained (the text, then its cost adjuster)
        if st == 'this.getSpellAbility().addEffect(new InfoEffect("this spell costs {1} less to cast if it\'s bargained"))':
            return
        if st == 'this.getSpellAbility().setCostAdjuster(IceOutAdjuster.instance)':
            bargain = [a for a in added if a.get('class') == 'BargainAbility']
            if len(bargain) != 1:
                raise Unsupported('IceOutAdjuster without bargain')
            bargain[0]['discount'] = 1
            return
        # deck-building rules (Hare Apparent) do nothing in a game
        if re.fullmatch(r'this\.getSpellAbility\(\)\.addEffect\(new InfoEffect\("a deck can have any number of cards named [^"]*"\)\)', st):
            return
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addEffect\((.*)\)', st)
        if m:
            spell['effects'].append(t.effect(m.group(1)))
            return
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.withFirstModeCost\((.*)\)', st)
        if m:
            spell['first_mode_cost'] = mode_cost(m.group(1))
            return
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addMode\((.*)\)', st)
        if m:
            modes.append(mode_expr(t, m.group(1)))
            return
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.getModes\(\)\.set(Min|Max)Modes\((\d+)\)', st)
        if m:
            spell['min_modes' if m.group(1) == 'Min' else 'max_modes'] = int(m.group(2))
            return
        # hints and flavor words are display; a watcher is the engine's own bookkeeping
        if re.fullmatch(r'this\.getSpellAbility\(\)\.(addHint|addWatcher|withFirstModeFlavorWord|setAbilityWord)\(.*\)', st):
            return
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addCost\((.*)\)', st)
        if m:
            spell.setdefault('costs', []).append(t.cost(m.group(1)))
            return
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.addTarget\((.*)\)', st)
        if m:
            spell['targets'].append(t.target(m.group(1)))
            return
        # kicked, the spell's target is another (`ConditionalTargetAdjuster(KickedCondition.ONCE, blueprint, kicked)`)
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.setTargetAdjuster\(new ConditionalTargetAdjuster\(KickedCondition\.ONCE,(.*)\)\)', st)
        if m and len(a := split_args(m.group(1))) == 2:
            spell['kicked_targets'] = ([t.target(a[0])], [t.target(a[1])])
            return
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.setTargetAdjuster\(new ConditionalTargetAdjuster\(GiftWasPromisedCondition\.TRUE,(.*)\)\)', st)
        if m and len(a := split_args(m.group(1))) == 2:
            spell['gift_targets'] = ([t.target(a[0])], [t.target(a[1])])
            return
        # "with mana value X or less": the spell's last target
        # Kozilek's Command: its creature target's mana value is at most X, its graveyard target up to X cards
        if st == 'this.getSpellAbility().setTargetAdjuster(KozileksCommandAdjuster.instance)':
            for mode in [spell] + modes:
                for tg in mode.get('targets', []):
                    if tg['class'] == 'TargetPermanent':
                        tg.setdefault('filter', []).append({'p': 'mana_value_x', 'cmp': 'OR_LESS'})
                    if tg['class'] == 'TargetCardInGraveyard':
                        tg['max_x'] = True
            return
        m = re.fullmatch(r'this\.getSpellAbility\(\)\.setTargetAdjuster\(new XManaValueTargetAdjuster\(ComparisonType\.(OR_LESS|EQUAL_TO)\)\)', st)
        if m and spell['targets']:
            spell['targets'][-1].setdefault('filter', []).append({'p': 'mana_value_x', 'cmp': m.group(1)})
            return
        # "you may exile any number of <filter> from your hand. This spell costs {2} less for each": its info static,
        # and the adjuster
        m = re.fullmatch(r'ExileCardsFromHandAdjuster\.addAdjusterAndMessage\(this,(\w+)\)', st)
        if m:
            added.append({'class': 'ExileCardsFromHandAdjusterAbility', 'java': 'SimpleStaticAbility',
                          'filter': t.filter(m.group(1))})
            return
        # an effect's outcome only steers the Java AI
        if re.fullmatch(r'\w+\.setOutcome\(Outcome\.\w+\)', st):
            return
        raise Unsupported('statement ' + st)



def mode_expr(t, arg):
    """`new Mode(effect).addEffect(effect).addTarget(target)` or a `Mode` variable -> {effects, targets}"""
    arg = arg.strip()
    chained, more = [], []
    # spree and tiered: the mode's additional cost (a flavor word is only text)
    arg = re.sub(r'\.withFlavorWord\("[^"]*"\)$', '', arg)
    cost = None
    if (n := re.fullmatch(r'(.*)\.withCost\((.*)\)', arg, re.S)) and split_args(n.group(2)) == [n.group(2).strip()]:
        arg, cost = n.group(1).strip(), mode_cost(n.group(2))
    while True:
        n = re.fullmatch(r'(new Mode\(.*\))\s*\.add(Target|Effect)\((.*)\)', arg, re.S)
        if not n or split_args(n.group(3)) != [n.group(3).strip()]:
            break
        arg = n.group(1)
        if n.group(2) == 'Target':
            chained = [t.target(n.group(3))] + chained
        else:
            more = [t.effect(n.group(3))] + more
    if arg in t.vars:
        mode = t.vars[arg]
    else:
        cls, args = t.new_expr(arg)
        if cls != 'Mode':
            raise Unsupported('mode ' + arg)
        mode = {'class': 'Mode', 'effects': [t.effect(a) for a in args], 'targets': []}
    out = {'effects': mode.get('effects', []) + more, 'targets': mode.get('targets', []) + chained}
    if cost:
        out['cost'] = cost
    return out


def mode_cost(arg):
    """a spree mode's mana cost: `new GenericManaCost(n)` or `new ManaCostsImpl<>("{…}")`"""
    arg = arg.strip()
    if (m := re.fullmatch(r'new GenericManaCost\((\d+)\)', arg)):
        return '{' + m.group(1) + '}'
    if (m := re.fullmatch(r'new ManaCostsImpl<>\("((?:\{[^"]*\})+)"\)', arg)):
        return m.group(1)
    raise Unsupported('mode cost ' + arg)


CHAPTERS = {'I': 1, 'II': 2, 'III': 3, 'IV': 4, 'V': 5, 'VI': 6}


def ability_statement(t, st, added):
    # `Condition condition = new …Condition(…)`: a local the abilities name
    m = re.fullmatch(r'(?:final )?Condition (\w+)=(new \w+Condition\(.*\)|\w+Condition\.\w+)', st, re.S)
    if m:
        t.conditions[m.group(1)] = m.group(2)
        return True
    # `ManaCostsImpl c = new ManaCostsImpl<>("{X}{G}")`, then X's least value set in a loop (Lair of the Hydra)
    m = re.fullmatch(r'ManaCostsImpl(?:<\w*>)? (\w+)=new ManaCostsImpl<>\(("[^"]*")\)', st)
    if m:
        t.vars[m.group(1)] = {'class': 'ManaCost', 'cost': m.group(2)[1:-1]}
        return True
    m = re.fullmatch(r'for\(Object cost : (\w+)\)\{if\(cost instanceof VariableManaCost\)\{\(\(VariableManaCost\)cost\)\.setMinX\((\d+)\);\s*break;\}\}(.*)', st, re.S)
    if m and m.group(1) in t.vars:
        t.min_x = int(m.group(2))
        rest = m.group(3).strip().rstrip(';')
        return ability_statement(t, rest, added) if rest else True
    """A constructor statement that builds or adds an ability (cards and tokens alike); False if it isn't one"""
    # a Saga: its lore counter as it enters (added to the card first), then a chapter ability per chapter
    m = re.fullmatch(r'SagaAbility (\w+)=new SagaAbility\(this(?:\.get(?:Left|Right)HalfCard\(\))?\)', st)
    if m:
        t.vars[m.group(1)] = {'class': 'SagaAbility', 'java': 'SagaAbility', 'max': 0}
        added.append({'class': 'EntersBattlefieldAbility', 'java': 'EntersBattlefieldAbility',
                      'effects': [{'class': 'AddCountersSourceEffect', 'counter': {'name': 'lore', 'amount': 1}}]})
        return True
    m = re.fullmatch(r'(\w+)\.addChapterEffect\(this(?:\.get(?:Left|Right)HalfCard\(\))?,(.*)\)', st)
    if m and t.vars.get(m.group(1), {}).get('class') == 'SagaAbility':
        args = split_args(m.group(2))
        chapters = []
        while args and (c := re.fullmatch(r'SagaChapter\.CHAPTER_(\w+)', args[0].strip())):
            chapters.append(CHAPTERS[c.group(1)])
            args = args[1:]
        if len(chapters) == 1:
            chapters.append(chapters[0])
        if len(chapters) != 2 or not args:
            raise Unsupported('statement ' + st)
        targets = []
        if args[-1].strip().startswith('new Target'):
            targets = [t.target(args[-1])]
            args = args[:-1]
        effects = [t.effect(a) for a in args]
        for n in range(chapters[0], chapters[1] + 1):
            added.append({'class': 'TriggeredAbility', 'java': 'ChapterTriggeredAbility', 'on': {'e': 'chapter', 'n': n},
                          'effects': effects, 'targets': targets})
        t.vars[m.group(1)]['max'] = max(t.vars[m.group(1)]['max'], chapters[1])
        return True
    m = re.fullmatch(r'(?:final )?\w+ (\w+)=(.*)', st) or re.fullmatch(r'(\w+)=(new .*)', st)
    if m:
        t.assign(m.group(1), m.group(2))
        return True
    # "for each opponent, up to one target … that player controls": in a duel, one target an opponent controls
    m = re.fullmatch(r'(\w+)\.setTargetAdjuster\(new ForEachPlayerTargetsAdjuster\(false,true\)\)', st)
    if m and m.group(1) in t.vars and len(t.vars[m.group(1)].get('targets', [])) == 1 \
            and t.vars[m.group(1)]['targets'][0]['class'] == 'TargetPermanent':
        target = t.vars[m.group(1)]['targets'][0]
        target['filter'] = [dict(p) for p in target['filter']] + [dict(OPPONENT)]
        return True
    # "target planeswalker that player controls" (the damaged player): in a duel, an opponent's
    m = re.fullmatch(r'(\w+)\.setTargetAdjuster\(new ThatPlayerControlsTargetAdjuster\(\)\)', st)
    if m and m.group(1) in t.vars and len(t.vars[m.group(1)].get('targets', [])) == 1 \
            and t.vars[m.group(1)]['targets'][0]['class'] == 'TargetPermanent':
        target = t.vars[m.group(1)]['targets'][0]
        target['filter'] = [dict(p) for p in target['filter']] + [dict(OPPONENT)]
        t.vars[m.group(1)]['set_target_pointer'] = False
        return True
    # "target card from that player's graveyard": in a duel, the opponent's graveyard
    m = re.fullmatch(r'(\w+)\.setTargetAdjuster\(new ThatPlayerControlsTargetAdjuster\(true\)\)', st)
    if m and m.group(1) in t.vars and len(t.vars[m.group(1)].get('targets', [])) == 1 \
            and t.vars[m.group(1)]['targets'][0]['class'] == 'TargetCardInGraveyard':
        target = t.vars[m.group(1)]['targets'][0]
        target['filter'] = [dict(p) for p in target['filter']] + [{'p': 'owner', 'who': 'opponent'}]
        t.vars[m.group(1)]['set_target_pointer'] = False
        return True
    m = re.fullmatch(r'(\w+)\.withInterveningIf\((.*)\)', st)
    if m and m.group(1) in t.vars and t.vars[m.group(1)].get('class') == 'TriggeredAbility':
        t.vars[m.group(1)]['condition'] = t.value('condition', m.group(2).strip())
        return True
    # a trigger that looks back at the battlefield as it was (dies triggers here always do)
    m = re.fullmatch(r'(\w+)\.setLeavesTheBattlefieldTrigger\(true\)', st)
    if m and m.group(1) in t.vars and t.vars[m.group(1)].get('class') == 'TriggeredAbility':
        return True
    # untap every target's permanents (Nissa, Genesis Mage's creatures and lands)
    m = re.fullmatch(r'(\w+)\.setTargetPointer\(new EachTargetPointer\(\)\)', st)
    if m and isinstance(t.vars.get(m.group(1)), dict) and t.vars[m.group(1)].get('class') == 'UntapTargetEffect':
        t.vars[m.group(1)]['class'] = 'UntapEachTargetEffect'
        return True
    m = re.fullmatch(r'(\w+)\.(?:set(?:Outcome|Text|TargetPointer|RuleAtTheTop|AbilityWord)|addHint|withFlavorWord|withFirstModeFlavorWord)\(.*\)', st)
    if m and m.group(1) in t.vars:
        return True
    m = re.fullmatch(r'(\w+)\.setCostAdjuster\(GhostfireBladeAdjuster\.instance\)', st)
    if m and m.group(1) in t.vars and t.vars[m.group(1)].get('class') == 'EquipAbility':
        t.vars[m.group(1)]['colorless_discount'] = 2
        return True
    m = re.fullmatch(r'(\w+)\.setCostAdjuster\(LegendaryCreatureCostAdjuster\.instance\)', st)
    if m and m.group(1) in t.vars and t.vars[m.group(1)].get('class') == 'ChannelAbility':
        t.vars[m.group(1)]['legendary_discount'] = True
        return True
    # a rules text the ability's cost adjuster does
    m = re.fullmatch(r'(\w+)\.addEffect\(new InfoEffect\("This ability costs \{1\} less to activate for each '
                     r'legendary creature you control"\)\)', st)
    if m and m.group(1) in t.vars and t.vars[m.group(1)].get('class') == 'ChannelAbility':
        return True
    # Elven Passage's second half, which ElvenPassageEffect does too
    m = re.fullmatch(r'(\w+)\.addEffect\(new ElvenPassageUntapLandEffect\(\)\)', st)
    if m and m.group(1) in t.vars:
        return True
    # a modal triggered ability: "choose one" of its effects and targets, or a mode's
    m = re.fullmatch(r'(\w+)\.addMode\((.*)\)', st)
    if m and m.group(1) in t.vars and t.vars[m.group(1)].get('class') == 'TriggeredAbility':
        t.vars[m.group(1)].setdefault('modes', []).append(mode_expr(t, m.group(2)))
        return True
    # a mana ability built with a mana cost: tapping is one more cost; "if any of that mana is spent on a creature
    # spell, it gains haste until end of turn" (Arena of Glory)
    m = re.fullmatch(r'(\w+)\.add(Effect|Cost)\((.*)\)', st)
    if m and t.vars.get(m.group(1), {}).get('class') == 'ColoredManaAbility' and t.vars[m.group(1)].get('no_tap'):
        holder = t.vars[m.group(1)]
        if m.group(2) == 'Cost' and m.group(3).strip() == 'new TapSourceCost()':
            holder['no_tap'] = False
            return True
        if m.group(2) == 'Effect' and m.group(3).replace(' ', '') == 'newManaSpentOnSpellGainsAbilityEffect(' \
                'StaticFilters.FILTER_SPELL_CREATURE,newGainAbilityTargetEffect(HasteAbility.getInstance(),' \
                'Duration.EndOfTurn,null,true),true)':
            holder['creature_spell_haste'] = True
            return True
    m = re.fullmatch(r'(\w+)\.add(Effect|Target|Cost)\((.*)\)', st)
    if m and m.group(1) in t.vars and 'class' in t.vars[m.group(1)]:
        holder = t.vars[m.group(1)]
        mana = holder['class'] in ('WhiteManaAbility', 'BlueManaAbility', 'BlackManaAbility', 'RedManaAbility',
                                   'GreenManaAbility', 'ColorlessManaAbility')
        if holder['class'] in KEYWORDS and m.group(2) != 'Effect' and not (mana and m.group(2) == 'Cost'):
            raise Unsupported(holder['class'] + ' with another ' + m.group(2).lower())
        if m.group(2) == 'Effect':
            # `doIfCostPaid.addEffect(e)`: e too, if it's paid
            holder.setdefault('also' if holder.get('class') == 'DoIfCostPaid' else 'effects', []).append(t.effect(m.group(3)))
        elif m.group(2) == 'Target':
            holder.setdefault('targets', []).append(t.target(m.group(3)))
        else:
            holder.setdefault('cost', []).append(t.cost(m.group(3)))
        return True
    m = re.fullmatch(r'(?:this\.)?addAbility\((.*)\)', st)
    if m:
        args = split_args(m.group(1))
        # `addAbility(ability, new SomeWatcher())`: the watcher is the engine's own bookkeeping
        if len(args) == 2 and re.fullmatch(r'new \w+Watcher\((ability\.getOriginalId\(\))?\)', args[1].strip()):
            args = args[:1]
        # splice onto Arcane: revealed from the hand as an Arcane spell is cast, its effects added to that spell
        if (e := re.fullmatch(r'new SpliceAbility\(SpliceAbility\.ARCANE,"(\{[^"]*\})"\)', ','.join(args).strip())):
            added.append({'class': 'SpliceAbility', 'java': 'SpliceAbility', 'subtype': 'Arcane', 'cost': e.group(1)})
            return True
        # evolve: a trigger on a creature of yours entering with greater power or toughness
        if ','.join(args).strip() == 'new EvolveAbility()':
            added.append({'class': 'TriggeredAbility', 'java': 'EvolveAbility', 'on': {'e': 'evolve'},
                          'effects': [{'class': 'EvolveEffect'}]})
            return True
        # evoke: the alternative cost, then its "when this enters, if evoked, sacrifice it" trigger
        # suspend: the special action, then its upkeep and last-counter triggers (a creature's haste isn't done yet)
        if (e := re.fullmatch(r'new SuspendAbility\((\d+),new ManaCostsImpl<>\("(\{[^"]*\})"\),this\)', ','.join(args).strip())):
            added.append({'class': 'SuspendAbility', 'java': 'SuspendAbility', 'time': int(e.group(1)), 'cost': e.group(2)})
            added.append({'class': 'TriggeredAbility', 'java': 'SuspendUpkeepAbility', 'optional': False,
                          'on': {'e': 'step_begins', 'step': 'upkeep', 'who': 'you', 'suspended': True},
                          'effects': [{'class': 'SuspendRemoveTimeCounterEffect'}]})
            added.append({'class': 'TriggeredAbility', 'java': 'SuspendPlayCardAbility', 'optional': False,
                          'on': {'e': 'suspend_last_counter'}, 'effects': [{'class': 'SuspendPlayCardEffect'}]})
            return True
        # dash: an alternative cost; cast that way, it gains haste and returns to hand at the next end step
        if (e := re.fullmatch(r'new DashAbility\(("\{[^"]*\}")\)', ','.join(args).strip())):
            added.append({'class': 'EvokeAbility', 'java': 'DashAbility', 'cost': e.group(1)[1:-1], 'keyword': 'Dash'})
            # its "as it enters" part, listed on its own (the engine does it as the dashed creature enters)
            added.append({'class': 'BlitzEntersAbility', 'java': 'EntersBattlefieldAbility'})
            return True
        # replicate: its cost, then the cast trigger that copies the spell
        if (e := re.fullmatch(r'new ReplicateAbility\("(\{[^"]*\})"\)', ','.join(args).strip())):
            added.append({'class': 'ReplicateAbility', 'java': 'ReplicateAbility', 'cost': e.group(1)})
            added.append({'class': 'TriggeredAbility', 'java': 'ReplicateTriggeredAbility', 'optional': False,
                          'on': {'e': 'cast_this', 'replicated': True}, 'effects': [{'class': 'ReplicateCopyEffect'}],
                          'rule': 'Replicate <i>(When you cast this spell, copy it for each time you paid its replicate '
                                  'cost. You may choose new targets for the copies.)</i>'})
            return True
        # River of Tears: "{T}: Add {U}. If <condition>, add {B} instead"
        if (e := re.fullmatch(r'new ConditionalManaAbility\(Zone\.BATTLEFIELD,new ConditionalManaEffect\(new BasicManaEffect\(Mana\.(\w+)Mana\(1\)\),new BasicManaEffect\(Mana\.(\w+)Mana\(1\)\),(\w+\.instance),("[^"]*")\),new TapSourceCost\(\)\)', ','.join(args).strip())):
            colors = {'White': 'W', 'Blue': 'U', 'Black': 'B', 'Red': 'R', 'Green': 'G'}
            if e.group(1) in colors and e.group(2) in colors:
                added.append({'class': 'ConditionalColorManaAbility', 'java': 'ConditionalManaAbility',
                              'color': colors[e.group(1)], 'otherwise': colors[e.group(2)],
                              'condition': t.condition(e.group(3)), 'rule': e.group(4)[1:-1]})
                return True
        if (e := re.fullmatch(r'new EvokeAbility\((new ExileFromHandCost\(.*\))\)', ','.join(args).strip())):
            added.append({'class': 'EvokeAbility', 'java': 'EvokeAbility', 'cost': '', 'costs': [t.cost(e.group(1))]})
            added.append({'class': 'TriggeredAbility', 'java': 'EntersBattlefieldTriggeredAbility',
                          'on': {'e': 'enters'}, 'condition': {'c': 'evoked'},
                          'effects': [{'class': 'SacrificeSourceEffect'}]})
            return True
        if (e := re.fullmatch(r'new EvokeAbility\(("\{[^"]*\}")\)', ','.join(args).strip())):
            added.append({'class': 'EvokeAbility', 'java': 'EvokeAbility', 'cost': e.group(1)[1:-1]})
            added.append({'class': 'TriggeredAbility', 'java': 'EntersBattlefieldTriggeredAbility',
                          'on': {'e': 'enters'}, 'condition': {'c': 'evoked'},
                          'effects': [{'class': 'SacrificeSourceEffect'}]})
            return True
        # a Siege: its static reminder, then its "when defeated, exile it and cast it transformed" trigger
        if ','.join(args).strip() == 'new SiegeAbility()':
            added.append({'class': 'SimpleStaticAbility', 'java': 'SiegeAbility', 'effects': [], 'rule': ''})
            added.append({'class': 'TriggeredAbility', 'java': 'SiegeDefeatedTriggeredAbility',
                          'on': {'e': 'siege_defeated'}, 'optional': True,
                          'effects': [{'class': 'SiegeDefeatedEffect'}]})
            return True
        # a Case: its static marker, the initial ability, the "to solve" end step trigger and the solved ability
        if (e := re.fullmatch(r'new CaseAbility\((.*)\)', t.strip_setters(','.join(args)).strip(), re.S)):
            parts = split_args(e.group(1))
            if len(parts) != 3:
                raise Unsupported('CaseAbility ' + e.group(1))
            added.append({'class': 'SimpleStaticAbility', 'java': 'CaseAbility', 'effects': [], 'rule': ''})
            added.append(t.ability(parts[0]))
            added.append({'class': 'TriggeredAbility', 'java': 'CaseSolveAbility',
                          'on': {'e': 'step_begins', 'step': 'end', 'who': 'you'},
                          'condition': {'c': 'and', 'xs': [t.value('condition', parts[1].strip()),
                                                           {'c': 'not', 'x': {'c': 'solved'}}]},
                          'effects': [{'class': 'SolveEffect'}]})
            solved = t.ability(parts[2])
            if solved.get('condition') != {'c': 'solved'}:
                raise Unsupported('a solved ability without the solved condition')
            added.append(solved)
            return True
        # exert: the static ability, then its "when you do" trigger
        if (e := re.fullmatch(r'new ExertAbility\((\w+|new BecomesExertSourceTriggeredAbility\(.*\))\)', ','.join(args).strip(), re.S)):
            added.append({'class': 'ExertAbility'})
            added.append(t.ability(e.group(1)))
            return True
        added.append(t.ability(','.join(args)))
        # blitz: its "as it enters" part, listed on its own
        if added[-1].get('class') == 'BlitzAbility':
            added.append({'class': 'BlitzEntersAbility', 'java': 'EntersBattlefieldAbility'})
        # foretell: its "look at the face-down card" static, listed on its own
        if added[-1].get('class') == 'ForetellAbility':
            added.append({'class': 'SimpleStaticAbility', 'effects': [], 'rule': ''})
        # offspring: its "when this enters, if its offspring cost was paid" trigger, listed on its own
        if added[-1].get('class') == 'OffspringAbility':
            added.append({'class': 'TriggeredAbility', 'java': 'EntersBattlefieldTriggeredAbility', 'on': {'e': 'enters'},
                          'condition': {'c': 'offspring'}, 'effects': [{'class': 'OffspringEffect'}]})
        return True
    return False


def in_java_order(java_abilities, added):
    """The abilities in Java's order (decision logs number them so), each matched to one the constructor added"""
    abilities = []
    for fact in java_abilities:
        if fact['class'] in IMPLICIT:
            abilities.append({'class': fact['class']})
            continue
        match = next((a for a in added if a.get('java', a['class']) == fact['class']), None)
        if match is None:
            raise Unsupported('ability ' + fact['class'] + ' (added by another ability)')
        added.remove(match)
        if match.pop('java', None) or match['class'] in ABILITIES:
            match['rule'] = fact['rule']
        abilities.append(match)
    if added:
        raise Unsupported('abilities Java does not list: ' + ', '.join(a['class'] for a in added))
    return abilities


def card_values(source):
    """`DynamicValue` fields (`private static final DynamicValue xValue = new …;`) -> their texts, by name"""
    return {m.group(1): canonical(m.group(2))
            for m in re.finditer(r'static final \w*(?:Value|Count) (\w+)\s*=\s*(.*?);', source, re.S)}


def card_conditions(source):
    """condition fields (`private static final Condition condition = new …;`) -> their texts, by name"""
    return {m.group(1): canonical(m.group(2))
            for m in re.finditer(r'static final \w*Condition (\w+)\s*=\s*(.*?);', source, re.S)}


def balanced(call):
    """whether `new X(…)`'s first parenthesis closes at its end (the text is one call, not a chain)"""
    depth, quoted = 0, False
    start = call.index('(')
    for i, ch in enumerate(call[start:], start):
        if ch == '"':
            quoted = not quoted
        elif not quoted and ch == '(':
            depth += 1
        elif not quoted and ch == ')':
            depth -= 1
            if depth == 0:
                return i == len(call) - 1
    return False


def token(cls):
    """A token class from Mage/…/permanent/token: name, types, colors, subtypes, power/toughness, abilities (in the
    order and with the rule texts of its facts, `token:<class>`)"""
    path = os.path.join(TOKENS, cls + '.java')
    if not os.path.exists(path):
        raise Unsupported('no token class ' + cls)
    source = open(path).read()
    m = re.search(r'public ' + cls + r'\(\)\s*\{(.*?)\n    \}', source, re.S)
    if not m:
        raise Unsupported('token constructor ' + cls)
    # `this(0)`: the X/X constructor with X = 0 (SharkToken); the engine sets X as it creates one
    if (n := re.fullmatch(r'\s*this\((\d+)\);\s*', m.group(1))):
        m = re.search(r'public ' + cls + r'\(int (\w+)\)\s*\{(.*?)\n    \}', source, re.S)
        if not m:
            raise Unsupported('token constructor ' + cls)
        m = re.match(r'(.*)', re.sub(r'\b' + m.group(1) + r'\b', n.group(1), m.group(2)), re.S)
    out = {'types': [], 'colors': [], 'subtypes': [], 'abilities': []}
    subtypes = subtype_names()
    colors = {'White': 'W', 'Blue': 'U', 'Black': 'B', 'Red': 'R', 'Green': 'G'}
    t = Translator(declared_filters(source, subtypes), card_conditions(source), subtypes)
    added = []
    for st in statements(m.group(1)):
        st = re.sub(r'^this\.(?=(?:cardType|color|subtype|power|toughness|startingLoyalty)\b)', '', st)
        if (n := re.fullmatch(r'super\("([^"]+)",".*"\)', st)):
            out['name'] = n.group(1)
        elif (n := re.fullmatch(r'(?:this\.)?cardType\.add\(CardType\.(\w+)\)', st)):
            out['types'].append(n.group(1))
        elif (n := re.fullmatch(r'color\.set(\w+)\(true\)', st)):
            out['colors'].append(colors[n.group(1)])
        elif (n := re.fullmatch(r'subtype\.add\(SubType\.(\w+)\)', st)):
            out['subtypes'].append(subtypes[n.group(1)])
        elif (n := re.fullmatch(r'(power|toughness)=new MageInt\((\d+)\)', st)):
            out[n.group(1)] = int(n.group(2))
        elif (n := re.fullmatch(r'(?:this\.)?startingLoyalty=(\d+)', st)):
            out['loyalty'] = int(n.group(1))
        elif not ability_statement(t, st, added):
            raise Unsupported('token statement ' + st)
    if added:
        if 'token:' + cls not in FACTS:
            raise Unsupported('no token facts ' + cls)
        out['abilities'] = in_java_order(FACTS['token:' + cls], added)
    if EXTRA.get('token:' + cls, {}).get('images'):
        out['images'] = EXTRA['token:' + cls]['images']
    out['class'] = cls
    return out


def balanced_args(source, start):
    """the text inside the parentheses that open at `start`"""
    depth, i = 0, start
    while i < len(source):
        if source[i] == '(':
            depth += 1
        elif source[i] == ')':
            depth -= 1
            if depth == 0:
                return source[start + 1:i]
        elif source[i] == '"':
            i = source.index('"', i + 1)
        i += 1
    raise Unsupported('unbalanced hint')


NAMED_HINTS = {'CanPlayAdditionalLandsHint', 'DayNightHint', 'ClassLevelHint', 'OpenSideboardHint',
               'OpponentsLostLifeHint', 'CreaturesYouControlHint', 'RaidHint', 'MyTurnHint', 'MorbidHint',
               'FerociousHint', 'LandsYouControlHint', 'BargainCostWasPaidHint', 'TeamworkCostWasPaidHint'}


def _cond_hint(c, text):
    return {'h': 'condition', 'condition': CONDITIONS[c], 'icons': True, 'trueText': text, 'falseText': text}


# the named hints that are a condition or a value hint (`RaidHint` is "You attacked this turn")
NAMED_AS = {
    'OpponentsLostLifeHint': _cond_hint('OpponentsLostLifeCondition.instance', 'An opponent lost life this turn'),
    'RaidHint': _cond_hint('RaidCondition.instance', 'You attacked this turn'),
    'MyTurnHint': _cond_hint('MyTurnCondition.instance', "It's your turn"),
    'MorbidHint': _cond_hint('MorbidCondition.instance', 'A creature died this turn'),
    'FerociousHint': _cond_hint('FerociousCondition.instance', 'You control a creature with power 4+'),
    'CreaturesYouControlHint': {'h': 'value', 'name': 'Creatures you control',
                                'value': {'v': 'permanents', 'filter': COUNT_FILTERS['CreaturesYouControlCount.PLURAL']}},
    'LandsYouControlHint': {'h': 'value', 'name': 'Lands you control',
                            'value': {'v': 'permanents', 'filter': COUNT_FILTERS['LandsYouControlCount.instance']}},
}


def card_hints(name, subtypes, abilities=None):
    """The hints a card's view shows in a game (`CardUtil.getCardRulesWithAdditionalInfo`), in Java's order: from its
    facts (each ability's hints, their texts), with the conditions and values from its source (`addHint(…)`);
    `abilities`: the facts of other abilities of the card (a room's halves')"""
    if abilities is None:
        abilities = EXTRA.get(name, {}).get('abilities', [])
    descs = [(a['class'], h) for a in abilities for h in a.get('hints', [])]
    if not descs:
        return []
    # hints an ability class adds itself, with nothing in the card's source
    own = []
    for ability, d in descs:
        if d['class'] in NAMED_AS:
            own.append(NAMED_AS[d['class']])
        elif d['class'] in NAMED_HINTS:
            own.append({'h': 'named', 'name': d['class']})
        elif ability in ABILITY_HINTS:
            own.append(dict(ABILITY_HINTS[ability](d), name=d.get('name', '')) if d['class'] == 'ValueHint'
                       else ABILITY_HINTS[ability](d))
        else:
            own.append(None)
    if all(o is not None for o in own):
        return own
    try:
        source = open(card_path(name)).read()
    except (Unsupported, OSError):
        return [{'h': 'unknown', 'of': d['class'], 'why': 'no source'} for d in descs]
    fields = {m.group(1): canonical(m.group(2)) for m in
              re.finditer(r'(?:private|public)\s+static\s+final\s+\w*Hint\s+(\w+)\s*=\s*(.*?);\n', source, re.S)}
    exprs = [canonical(balanced_args(source, m.end() - 1)) for m in re.finditer(r'\baddHint\(', source)]
    exprs = [fields.get(e, e) for e in exprs]
    t = Translator(declared_filters(source, subtypes), card_conditions(source), subtypes, card_values(source))
    out = []
    for (ability, d), o in zip(descs, own):
        cls = d['class']
        if o is not None:
            if exprs and exprs[0] == cls + '.instance':
                exprs.pop(0)
            out.append(o)
            continue
        expr = exprs.pop(0) if exprs else ''
        try:
            out.append(hint_of(t, expr, d))
        except Unsupported as e:
            out.append({'h': 'unknown', 'of': cls, 'why': str(e)})
    return out


def hint_of(t, expr, d):
    texts = {k: d[k] for k in ('trueText', 'falseText', 'trueColor', 'falseColor') if k in d}
    icons = d.get('useIcons', 'true') == 'true'
    if (m := re.fullmatch(r'new (?:ConditionHint|ConditionPermanentHint)\((.*)\)', expr, re.S)):
        cond = split_args(m.group(1))[0]
        return dict({'h': 'condition', 'condition': t.condition(cond), 'icons': icons}, **texts)
    if (m := re.fullmatch(r'new ValueHint\((".*?"),(.*)\)', expr, re.S)):
        return {'h': 'value', 'name': d.get('name', json.loads(m.group(1))), 'value': t.amount(m.group(2))}
    if (m := re.fullmatch(r'(.+)\.getHint\(\)', expr, re.S)):
        inner = m.group(1)
        if d['class'] == 'ValueHint':
            if inner in HINT_VALUES:
                return {'h': 'value', 'name': d['name'], 'value': HINT_VALUES[inner]}
            for v in (inner, inner + '.instance'):
                try:
                    return {'h': 'value', 'name': d['name'], 'value': t.amount(v)}
                except Unsupported:
                    pass
            raise Unsupported('hint value ' + inner)
        for c in (inner, inner + '.instance'):
            try:
                return dict({'h': 'condition', 'condition': t.condition(c), 'icons': icons}, **texts)
            except Unsupported:
                pass
        raise Unsupported('hint condition ' + inner)
    if d['class'] == 'CardTypesInGraveyardHint' or expr == 'CardTypesInGraveyardCount.ALL.getHint()':
        return {'h': 'value', 'name': 'Card types in graveyards', 'value': {'v': 'card_types_in_graveyards'}}
    raise Unsupported('hint ' + expr[:80])


NOT_GAINED_LIFE = {'c': 'not', 'x': {'c': 'you_gained_life'}}
YOURS = {'p': 'controller', 'who': 'you'}
# the values whose `getHint()` a card adds (`LessonsInGraveCondition.getHint()` counts Lessons)
HINT_VALUES = {
    'LessonsInGraveCondition': {'v': 'cards_in_graveyard', 'filter': [{'p': 'subtype', 's': 'Lesson'}]},
    'ControllerGainedLifeCount': {'v': 'life_gained'},
    'LegendaryCreatureCostAdjuster': {'v': 'permanents', 'filter': [{'p': 'type', 't': 'CREATURE'},
                                                                    {'p': 'supertype', 's': 'LEGENDARY'}, YOURS]},
}
# the hints an ability class adds (by its facts): `DelveAbility` counts your graveyard…
ABILITY_HINTS = {
    'AffinityAbility': lambda d: {'h': 'value', 'value': {'v': 'permanents', 'filter': [
        {'p': 'subtype', 's': d['name'].removesuffix(' you control').removesuffix('s')}, YOURS]}},
    'AffinityForArtifactsAbility': lambda d: {'h': 'value', 'value': {'v': 'permanents', 'filter': [
        {'p': 'type', 't': 'ARTIFACT'}, YOURS]}},
    'DelveAbility': lambda d: {'h': 'value', 'value': {'v': 'cards_in_graveyard', 'filter': []}},
    'ConvokeAbility': lambda d: {'h': 'value', 'value': {'v': 'permanents', 'filter': [
        {'p': 'type', 't': 'CREATURE'}, YOURS, {'p': 'tapped', 'v': False}]}},
    'DrawNthCardTriggeredAbility': lambda d: {'h': 'value', 'value': {'v': 'cards_drawn'}},
    'ExpendTriggeredAbility': lambda d: {'h': 'value', 'value': {'v': 'mana_expended'}},
    'GainLifeFirstTimeTriggeredAbility': lambda d: {'h': 'condition', 'condition': NOT_GAINED_LIFE, 'icons': True,
                                                    'trueText': d['trueText'], 'falseText': d['falseText']},
    'CatCollectorTriggeredAbility': lambda d: {'h': 'condition', 'condition': NOT_GAINED_LIFE, 'icons': True,
                                               'trueText': d['trueText'], 'falseText': d['falseText']},
}


def school_pool():
    names = set()
    for f in glob.glob(os.path.join(ROOT, 'Mage.Server', 'career-src', 'pioneer', '*.txt')) + \
            glob.glob(os.path.join(ROOT, 'Mage.Server', 'career-src', 'standard', '*.txt')) + \
            glob.glob(os.path.join(ROOT, 'rust', 'crates', 'playmat-engine', 'shadow-decks', '*', '*.txt')):
        for line in open(f):
            line = line.strip()
            m = re.match(r'(?:SB:\s*)?(\d+)\s+(.+?)(\s+\(.*)?$', line)
            if line and not line.startswith('#') and m:
                names.add(m.group(2).strip())
    return sorted(names)


# report every unsupported statement of a card, not just the first (PLAYMAT_ALL_ERRORS=1)
ALL_ERRORS = os.environ.get('PLAYMAT_ALL_ERRORS') == '1'

# every card's and token's facts, by name (tokens as `token:<class>`)
FACTS = {}
# the rest of each fact (a card's set, a token's images)
EXTRA = {}


def card_path(card_name):
    cls = class_name(card_name)
    path = os.path.join(SETS, cls[0].lower(), cls + '.java')
    if not os.path.exists(path) and ' // ' in card_name:
        # a split card (a Room) is named after both halves
        cls = ''.join(class_name(n) for n in card_name.split(' // '))
        path = os.path.join(SETS, cls[0].lower(), cls + '.java')
    return path


EMBLEMS = os.path.join(ROOT, 'Mage', 'src', 'main', 'java', 'mage', 'game', 'command', 'emblems')


def emblem(cls):
    """An emblem class from Mage/…/command/emblems: its abilities (with the rule texts of its facts, `emblem:<class>`)"""
    path = os.path.join(EMBLEMS, cls + '.java')
    if not os.path.exists(path):
        raise Unsupported('no emblem class ' + cls)
    source = open(path).read()
    m = re.search(r'public ' + cls + r'\(\)\s*\{(.*?)\n    \}', source, re.S)
    if not m:
        raise Unsupported('emblem constructor ' + cls)
    subtypes = subtype_names()
    filters = declared_filters(source, subtypes)
    # filters the constructor declares for itself
    body = m.group(1)
    for f in re.finditer(r'(?:final )?Filter\w+ (\w+)\s*=\s*new (Filter\w+)\((.*?)\);', body, re.S):
        adds = [canonical(p) for p in re.findall(r'\b' + f.group(1) + r'\.add\((.*?)\);', body, re.S)]
        filters[f.group(1)] = filter_object(f.group(2), split_args(canonical(f.group(3))), adds, subtypes)
    t = Translator(filters, card_conditions(source), subtypes)
    out = {'types': [], 'abilities': []}
    added = []
    for st in statements(body):
        if (n := re.fullmatch(r'(?:final )?Filter\w+ (\w+)=new Filter.*', st) or re.fullmatch(r'(\w+)\.add\(.*\)', st)) \
                and n.group(1) in filters:
            continue
        st = re.sub(r'^this\.getAbilities\(\)\.add\((.*)\)$', r'this.addAbility(\1)', st)
        st = re.sub(r'^getAbilities\(\)\.add\((.*)\)$', r'this.addAbility(\1)', st)
        if (n := re.fullmatch(r'super\("([^"]+)"\)', st)):
            out['name'] = n.group(1)
        elif not ability_statement(t, st, added):
            raise Unsupported('emblem statement ' + st)
    if 'emblem:' + cls not in FACTS:
        raise Unsupported('no emblem facts ' + cls)
    out['abilities'] = in_java_order(FACTS['emblem:' + cls], added)
    if EXTRA.get('emblem:' + cls, {}).get('images'):
        out['images'] = EXTRA['emblem:' + cls]['images']
    return out


def emblem_classes(names):
    """The emblem classes the pool's cards give (`new XEmblem()`), for the facts"""
    out = set()
    for name in names:
        try:
            source = open(card_path(name)).read()
        except (Unsupported, OSError):
            continue
        out.update(m.group(1) for m in re.finditer(r'new (\w+Emblem)\(\)', source)
                   if os.path.exists(os.path.join(EMBLEMS, m.group(1) + '.java')))
    return sorted(out)


def token_classes(names):
    """The token classes the pool's cards create (`new XToken()`), for the facts"""
    out = set()
    for name in names:
        try:
            source = open(card_path(name)).read()
        except (Unsupported, OSError):
            continue
        for m in re.finditer(r'new (\w+Token)\((?:\w+)?\)|(\w+Token)::new', source):
            cls = m.group(1) or m.group(2)
            if os.path.exists(os.path.join(TOKENS, cls + '.java')):
                out.add(cls)
        if 'InvestigateEffect' in source:
            out.add('ClueArtifactToken')
        # tokens a keyword makes
        if 'MobilizeAbility' in source:
            out.add('RedWarriorToken')
        if 'EmpowerJaceEffect' in source:
            out.add('JacePlaneswalkerToken')
        for role in re.findall(r'RoleType\.(\w+)', source):
            out.add(role.title().replace('_', '') + 'RoleToken')
    return sorted(out)


def granted_rules(card):
    """a granted ability without its own rule text: the quoted part of the granting ability's (a ward, "ward {N}")"""
    for a in card.get('abilities', []):
        for e in a.get('effects', []):
            g = e.get('ability') if isinstance(e, dict) else None
            if not isinstance(g, dict) or 'rule' in g:
                continue
            quoted = re.search(r'"([^"]*)"', a.get('rule', ''))
            cost = [c.get('cost') for x in g.get('effects', []) for c in x.get('cost', []) if c.get('class') == 'ManaCost']
            if g.get('class') == 'WardAbility' and len(cost) == 1:
                # `WardAbility.getRule`: with its hint unless it's turned off
                g['rule'] = 'ward ' + cost[0]
                if g.get('hint'):
                    g['rule'] += (' <i>(Whenever this creature becomes the target of a spell or ability an opponent'
                                  ' controls, counter it unless that player pays ' + cost[0] + '.)</i>')
            elif g.get('class') == 'SimpleActivatedAbility' and quoted:
                # an activated ability's rule ends with its effect's sentence
                g['rule'] = quoted.group(1).rstrip('.') + '.'
            elif g.get('class') == 'TriggeredAbility' and quoted:
                # Java writes the granted trigger's own text, where the creature is {this}
                g['rule'] = re.sub(r'\b[Tt]his creature\b', '{this}', quoted.group(1))


def strip_java(x):
    """the Java class names kept while translating (a granted ability's too): the engine doesn't read them"""
    if isinstance(x, dict):
        x.pop('java', None)
        if x.get('class') == 'WardAbility':
            x.pop('hint', None)
        for v in x.values():
            strip_java(v)
    elif isinstance(x, list):
        for v in x:
            strip_java(v)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--pool')
    parser.add_argument('--write-facts-pool', help='write the CardFactsTest pool (cards, then token:<class>) and stop')
    parser.add_argument('--facts', default=DEFAULT_FACTS)
    parser.add_argument('--out', default=DEFAULT_OUT)
    parser.add_argument('--report')
    args = parser.parse_args()
    pool = [l.strip() for l in open(args.pool)] if args.pool else school_pool()
    if args.write_facts_pool:
        with open(args.write_facts_pool, 'w') as f:
            f.write('\n'.join(pool + ['token:' + t for t in token_classes(pool)]
                              + ['emblem:' + e for e in emblem_classes(pool)]) + '\n')
        return
    subtypes = subtype_names()
    facts = {c['name']: c['abilities'] for c in json.load(open(args.facts))}
    FACTS.update(facts)
    EXTRA.update({c['name']: c for c in json.load(open(args.facts))})
    cards, report = [], []
    for name in pool:
        if not name:
            continue
        try:
            card = translate(name, subtypes, facts)
            if EXTRA.get(card['name'], {}).get('set'):
                card['set'] = EXTRA[card['name']]['set']
            granted_rules(card)
            hints = card_hints(card['name'], subtypes)
            if hints:
                card['hints'] = hints
                # each hint's ability, by its index in the card's abilities
                card['hint_abilities'] = [i for i, a in enumerate(EXTRA.get(card['name'], {}).get('abilities', []))
                                          for _ in a.get('hints', [])]
            # a room's halves' hints, from one pass over the card's source (`addHint` calls in the halves' order)
            halves = EXTRA.get(card['name'], {}).get('halves', [])
            if card.get('room') and len(halves) == len(card['room']):
                hints = card_hints(card['name'], subtypes, [a for h in halves for a in h.get('abilities', [])])
                for half, facts_half in zip(card['room'], halves):
                    n = sum(len(a.get('hints', [])) for a in facts_half.get('abilities', []))
                    if hints[:n]:
                        half['hints'] = hints[:n]
                        half['hint_abilities'] = [i for i, a in enumerate(facts_half.get('abilities', []))
                                                  for _ in a.get('hints', [])]
                    hints = hints[n:]
            strip_java(card)
            cards.append(card)
        except Unsupported as e:
            report.append('%s: %s' % (name, e))
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, 'w') as f:
        f.write('[\n' + ',\n'.join(json.dumps(c, sort_keys=True) for c in sorted(cards, key=lambda c: c['name'])) + '\n]\n')
    if args.report:
        with open(args.report, 'w') as f:
            f.write('\n'.join(report) + '\n')
    print('%d of %d cards translated' % (len(cards), len([n for n in pool if n])), file=sys.stderr)


if __name__ == '__main__':
    main()
