# Playtesting

## Highest-value test

Give the game to someone with no live explanation and observe.

Do not rescue every hesitation immediately. Confusion is product data.

The important question is not “did the command execute?” but whether the game creates understanding, curiosity, choice and a desire to continue.

Widen the playtest audience progressively. Developer-only and close-friend testing is useful for exposing obvious interaction failures; broader testing becomes valuable when the current slice is coherent enough that unexplained confusion is meaningful product evidence rather than a known unfinished surface.

## Observe

- Can they start without external instructions?
- Do they understand where they are and what the immediate activity is?
- Is the primary action obvious and are its consequences safe/clear?
- Do they notice weather/time/bait/gear clues naturally?
- When something bites, do they understand that the observation matters?
- Can they recover when bait runs out?
- Can they navigate back from a deep screen?
- Do they explore voluntarily?
- Do they care about an individual specimen or record?
- Do they form theories about hidden conditions/bait/species?
- Do they notice/remember Mara as a character rather than a menu label?
- In a group, will they press the shared action without being reassured verbally?
- Do they understand what is public versus ephemeral/private?
- Does visual art make them care about locations/characters/discoveries more?
- Would they want to share a catch/art/profile into another chat?

## Strong signals

- “What else can I catch/find?”
- “Does this bait change something?”
- “Why did that only appear in fog?”
- “I want to beat that record.”
- “What is behind that place?”
- “Can I talk to her again?”
- “Can I unlock that picture/outfit?”
- players comparing theories or showing each other discoveries without prompting.

## UX failure signals

- the player cannot explain what a screen/action is;
- they hesitate because cost/consequences are mysterious rather than because the world is mysterious;
- `/help` reads like documentation instead of onboarding;
- slash commands become the practical navigation because buttons are insufficient;
- social activity immediately forces everyone into DMs;
- public group output becomes noisy;
- consumables create a soft lock;
- generated art looks inconsistent enough that a recurring character is not recognizable;
- rich imagery makes controls/state harder to find.

## Balance testing

Tune economy/probabilities only after the UI communicates the underlying choice. A confusing bait screen makes bait-balance feedback meaningless.

Separate:

- onboarding friction;
- information discoverability;
- actual probability/difficulty;
- economy pacing;
- technical latency/failure.

## Timing instrumentation

Reaction windows should be tuned from real transport/play data, not intuition alone. When instrumentation is expanded, useful fields include cast time, bite due/presentation time, callback receipt time, reaction category, species, location, bait/equipment, outcome and failure reason. The purpose is aggregate timing/game-balance insight; do not log credentials or complete Telegram message text by default.

When tuning encounter selection, developer-only diagnostics should be able to explain *why* a result was possible/chosen: eligible candidates, rejected candidates/reasons, base and modified weights, relevant conditions, random roll/domain/seed, and timing boundaries. That explainability is for debugging/balancing; the normal player experience should still preserve imperfect information rather than exposing raw formulas.

## New systems

For narrative/relationship/art systems, test emotional and memory outcomes, not only completion:

- Can the player describe an NPC's personality after a few interactions?
- Do choices feel recognized later?
- Does a reputation/relationship change produce something more interesting than a bar increment?
- Does unlocking art feel attached to the accomplishment that earned it?
- Is a shareable card understandable outside the player's private game context?
