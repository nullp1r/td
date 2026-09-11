# Character identity and onboarding

## Locked: newcomer framing

The first playable character is a newcomer to the starting environment. The exact reason for arriving, prior biography, era/technology framing and transportation mechanism remain open.

The character should not begin as a chosen one, established local authority or predefined fishing professional.

## Leaning: minimal creation before world entry

Current preferred flow:

```text
/start
  ↓
account exists/created
  ↓
character draft
  ↓
choose/confirm in-world name
  ↓
explicit world-entry transition
```

Avoid blocking entry on appearance, profession, class, biography or other choices whose gameplay meaning is not yet established. Pronouns/appearance/background can be introduced when dialogue, localization or actual mechanics give the choice meaning rather than as mandatory form fields.

Character IDs are stable internal identity. **Open:** duplicate-name policy, rename policy and how much Telegram identity is shown when disambiguation is necessary.

Keep three onboarding concepts distinct:

- **account onboarding** — Telegram routing/preferences/account creation;
- **character onboarding** — establish minimum in-world identity and enter the world;
- **world onboarding** — the much longer process of learning movement, items, people, trade, danger, knowledge and cooperation through play.

## Leaning: explicit world-boundary entry points

A new character should not materialize in the middle of a public street. The current design candidate is an explicit world-boundary entry point: arrivals hall, terminal, road gate, landing, station entrance or another place that plausibly connects the modeled region to an unmodeled outside world.

A brief personal arrival presentation may precede physical entry, but it owns no private simulation. It can read canonical time/weather/conditions for presentation.

The actual entry transition should be atomic:

```text
creating + no location
    ↓
active + entry location + starting state + CharacterEnteredWorld event
```

If the player closes Telegram afterward, the character remains at that canonical location. Reopening the bot is a session event, not another fictional arrival.

Character creation does not need to become a public spectacle. Existing players may simply see a newcomer present or “arrived recently”; the simulation does not need to materialize a special ferry/vehicle event every time somebody creates an account.

**Open:** exact first entry-point fiction and transport. Do not encode a ferry timetable merely to justify account creation at arbitrary real-world times.

### Design history: arrival fiction that is not canon

Several concrete openings were explored: ferry arrival, interrupted train, roadside breakdown and late-night arrival. Ferry was the strongest narrative candidate for a while, but none is locked.

The fixed script “arrive near evening / weather turning / last transport cancelled / leave tomorrow morning” is specifically **rejected as a universal onboarding fact** because it would imply player-owned time/weather or force implausible scheduling. Those details may occur only when canonical world state actually makes them true.

The durable architectural idea is the boundary/ingress, not a particular vehicle.

## Rejected: private tutorial Rustwater

A newcomer does not own a separate copy of:

- time;
- weather;
- tide;
- NPC positions;
- shop opening state;
- world events;
- other characters.

Two new characters entering seconds apart see the same canonical external world conditions, subject only to differences in character knowledge/personal state.

## Leaning: adaptive onboarding milestones

Prefer invisible experience milestones over a fixed tutorial quest chain. Examples:

```text
identity established
entered shared world
understood movement
interacted with an NPC
obtained an item
inspected an item
observed player presence
encountered an opportunity
used currency/trade
```

Different world interactions can satisfy the same milestone. Guidance should intervene when a concept has failed to surface naturally, rather than phasing canonical state so that a tutorial script can proceed.

The starter region should be a real persistent region optimized for legibility, safety and opportunity density. It should remain relevant to experienced characters so newcomers can encounter actual MMO activity.

## Open conflict: Mara's first meeting

Earlier design direction wanted Mara to meet the player near the beginning and for their first encounter to be the characters' genuine first meeting.

Later shared-world reasoning established that named NPCs should not teleport or privately phase merely to satisfy onboarding, and explicitly allowed a newcomer to encounter someone else first when Mara is canonically elsewhere.

These positions conflict. Do **not** silently resolve the conflict by giving the newcomer a private Mara. The next NPC/onboarding design must decide whether:

- Mara's canonical schedule/presence can make an early meeting reliably believable;
- first contact can occur remotely/indirectly while preserving her physical state;
- or the requirement that every newcomer meet Mara immediately should be dropped.

Until that is decided, neither “Mara must always be the first NPC” nor “Mara may be absent indefinitely” is a locked target.

## Leaning: non-prescriptive starting state

A new character should begin with a small believable general-purpose kit and modest currency rather than a profession loadout. In particular, do not hand every newcomer a fishing rod/bait merely because fishing existed first.

Starting objects should establish that inventory and money exist without dictating what the character must become. Exact items and amounts remain content/balance decisions.

## Leaning: preserve acquisition context

Telegram-native onboarding should remember what caused a non-player to begin. Candidate entry contexts include:

- direct `/start`;
- shared item/object;
- character profile;
- party/expedition invitation;
- trade offer;
- world event/shared card.

That context may create personal reminders/pending invitations after entry without mutating canonical world conditions.

## Open

- Exact first-minute copy and first entry-point layout.
- Character-name uniqueness/disambiguation/rename policy.
- Starting inventory and starting currency amount.
- Whether an arrivals interior is a distinct location or only an entry presentation/POI.
- How much explicit “you are entering the game” language is useful versus fully diegetic presentation.
- Which onboarding milestones deserve persisted state versus derivation from normal character history.
