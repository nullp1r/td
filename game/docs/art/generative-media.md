# Generative media and collectible art

## Direction

Curated generative imagery is expected to become a **first-class content and presentation layer** for Rustwater. Many substantive game screens will likely be image-backed when art adds place, character, atmosphere, discovery or emotion, rather than relying only on text/Rich Message structure. Trivial confirmations do not need decorative images merely for consistency.

This is not merely marketing art. Visual content can communicate world state, character state, mystery, rewards and collection progress.

## Likely asset families

- location vistas;
- weather/time-of-day location variants;
- NPC portraits and expressions;
- outfit/event/season character variants;
- dialogue/story scenes and CGs;
- notable catch/specimen art;
- relic/item discoveries;
- event/social cards;
- achievements/titles/trophies;
- postcards and lore illustrations;
- profile/mascot/promo material;
- future biome/dimension/vehicle/faction art.

Not every utility screen needs an image. Dense inventory/record data should use imagery only where it improves comprehension or attachment.

## Art unlocks as gameplay

Unlocking images can itself be a reward/collection system.

Possible triggers:

- discover a location/species/relic;
- reach an NPC relationship milestone;
- learn a lore event;
- catch an exceptional/rare specimen;
- complete an objective/achievement;
- equip/earn a title;
- participate in a seasonal/social event;
- witness a rare weather/world state;
- complete a story branch.

Unlocked pieces can live in a gallery/journal/postcard collection and become shareable social objects.

A particularly strong Telegram loop is:

**do something meaningful → unlock a beautiful piece → share it through inline mode/Guest Mode into another conversation**.

Exact trading/provenance/rarity rules for art remain open.

## Production model

The preferred default for important/core assets is **offline generation + human curation + committed content metadata**.

Reasons:

- character consistency;
- art-direction control;
- predictable quality;
- no runtime generation latency/cost;
- moderation and lore continuity;
- deterministic unlocks;
- easier cropping/layout testing across Telegram clients.

Runtime generation may become useful later for personalized/procedural content, but it should be introduced only with a concrete experience that benefits from it.

## Game-state boundary

Authoritative game state should refer to **logical asset/unlock IDs**, not treat generated image bytes or a model prompt as gameplay truth.

A future asset record may include:

- stable asset ID;
- content/character/location association;
- unlock condition ID;
- source/reference image IDs;
- prompt/version/model metadata;
- aspect ratio and safe crops;
- variant tags (weather, expression, outfit, spoiler state, etc.);
- alt/fallback text;
- publication/moderation status;
- Telegram file identifiers/cache metadata where useful.

Do not design a database schema for all of this until the first real image-backed feature requires it; these are preservation requirements for the eventual pipeline.

## Character consistency

For recurring characters:

1. maintain a canonical character document/model sheet;
2. keep fixed recognition anchors explicit;
3. use approved reference images when the generation system supports them;
4. define controlled expression/outfit variants;
5. store prompts/references/metadata for reproducibility;
6. curate output manually;
7. reject attractive images that violate character identity.

Mara's canonical specification lives in [`../narrative/mara-reed.md`](../narrative/mara-reed.md).

## Visual consistency does not mean sameness

The world should support weather, cultures, distant regions, dimensions and increasingly strange content. A rigid single palette/style pasted everywhere would fight the long-term vision.

Consistency should come from:

- recurring character anchors;
- environmental storytelling quality;
- believable local design language;
- typography/UI treatment;
- intentional color scripts;
- art curation;
- recognizable Rustwater brand motifs where appropriate.

## Rich Messages + images

Telegram's 2026 Rich Messages support media blocks and buttons, making image-led game panels a natural evolution of the current UI. `tdx` already exposes helpers for photo/video/animation/audio/document/voice-note blocks, collage/slideshow and maps.

The future screen model can therefore be something closer to a compact illustrated game card than “an image attachment followed by bot commands.”

Maintain fallback text/structure where needed so a failed/unavailable image does not make the game state incomprehensible.

## AI-art quality rule

“Generated” is a production method, not a quality standard.

Every canonical asset should be judged for:

- character continuity;
- anatomy/objects/text artifacts;
- world/lore consistency;
- composition at the actual Telegram crop/size;
- spoiler appropriateness;
- rights/policy suitability for the intended use;
- whether it looks like Rustwater rather than generic AI/mobile-game art.

## Prompt architecture

Use three layers:

1. **stable brand/world block** — [`visual-direction.md`](visual-direction.md);
2. **stable character/object block** when recurring identity matters — e.g. Mara's canonical fragment;
3. **asset-specific block** — scene, action, emotion, weather, camera, aspect/crop and usage.

This is more reliable than rewriting the entire game premise differently for every asset.
