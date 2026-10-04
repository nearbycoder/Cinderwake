# Cinderwake sprite generation

Generated with the built-in ImageGen tool, October 4, 2026. No API/CLI fallback was used. Original source PNGs are preserved. The runtime imports connected alpha silhouettes, normalizes foot pivots and selects frames with a separate animation controller.

Selected assets: wanderer-v2.png (32 player poses), guardians-v1.png (32 enemy poses), regent-v1.png (8 boss poses). wanderer-v1.png is the earlier source retained for traceability.

## Player generation

Use case: stylized-concept.
Asset type: production sprite animation atlas for an original side-scrolling Rust action roguelite named Cinderwake.
Create one transparent PNG sprite sheet, exact regular grid of 8 columns and 4 rows, 32 evenly spaced frames, landscape 1536x1024 canvas. Each cell is 192x256. No text, no labels, no grid lines, no ground, no contact shadows, no background pixels. The entire background is genuinely alpha transparent.
Subject: the SAME agile adult human-proportioned masked brass wanderer in every frame, facing RIGHT in exact side view. Small angular bronze mask with one bright mint visor slit, layered teal cloth coat, dark fitted trousers, articulated bronze gauntlets and greaves, long burnt-orange scarf trailing left. Slender athletic anatomy, not a block robot, not chibi. One slender curved steel sabre. Original character.
Style: exquisite 2D pixel sprite art with the fluid anatomy, fine irregular pixel clusters, vibrant hue-shifted shading and painterly cel-shaded materials of a premium modern action roguelite. Detailed readable silhouette, 3-4 discrete tones per material, selective dark outline and mint/amber rim pixels. No smooth vector shapes, no large flat rectangular body parts, no toy-like chunky forms, no blurry painting, no glossy 3D look.
Layout: every figure has same scale: approximately 150 pixels tall in a 192x256 cell. Pelvis centered on x=96 within each cell. Every grounded pose's lowest boot pixel at y=220 within its cell. Keep every scarf, weapon and motion pose inside the cell with ample transparent gutter. Do NOT rescale individual poses. One character only per cell.
Row 1 frames left to right: 8-frame looping RUN cycle, clear contrasting contact, recoil, passing and flight poses, scarf follows movement; sabre carried low and safely within cell.
Row 2: 8-frame continuous SWORD SLASH action, anticipation, windup behind shoulder, forward swing, contact, follow-through, recovery. Articulate torso, shoulders and arms; distinct frame-to-frame poses. NO painted slash arcs or special effects because the engine adds them.
Row 3: first four frames are IDLE breathing cycle (subtle shoulders, scarf and grip motion); last four frames are JUMP takeoff, rising, apex, falling. Each jump body keeps the same scale.
Row 4: first four frames are DODGE ROLL crouch, tuck, sideways somersault, rising; fifth frame is PARRY guarding with blade; sixth frame is DRINKING a small flask; seventh frame is HURT recoil; eighth frame is DEATH collapsed low on the same baseline.
Accuracy priority: 8 columns, 4 rows, every frame centered inside its own equal cell; consistent identity and body scale; transparent empty gutters; all frames face right. Make this an actual usable atlas, not a concept art contact sheet.

## Player refinement

Edit target: the attached Cinderwake 32-frame sprite atlas.
Keep the exact character design, costume colors, right-facing orientation, action order and 8-column by 4-row grid.
Fix ONLY sprite packing and pixel-art finish:
- Maintain 1536x1024 canvas and 192x256 equal cells.
- Reduce ALL characters and their equipment uniformly by about 20% so no saber/scarf touches or crosses a cell boundary. Every single frame must have at least 12 pixels of transparent padding on left and right. Especially fix the sword-attack frames in row 2, which currently cross neighboring cells.
- Each frame grounded boot/contact point lies on y=218 within its own cell. Main torso centered at x=96. Preserve identical anatomy and scale across frames. Do not independently enlarge any crouched or collapsed pose.
- Convert the fine painted surface into crisp deliberate PIXEL ART: small hard-edged pixel clusters, 3-4 cel-shading bands per material, no soft gradient texture or grain, no blur. Keep slender fluid proportions and rich armor/cloth details.
- Preserve real transparent alpha background with entirely empty gutters. No opaque background, no colored haze, no drop shadows, no text or labels.
Exact row order remains: row1 eight run poses; row2 eight sequential slash poses; row3 four idle then four jump poses; row4 four dodge/roll then parry, drink, hurt, collapsed death.

## Guardian generation

Use case: stylized-concept.
Asset type: transparent sprite animation atlas for original 2D dark fantasy action roguelite Cinderwake.
Create a usable transparent PNG, 1536x1024, exact regular grid 8 columns x 4 rows, 32 sprites. Each cell 192x256. No titles, labels, lines, scenery, ground or contact shadows. True transparent alpha in every empty pixel. No colored fog or glow behind sprites.
Style: premium modern pixel-sprite art like the richly shaded animated characters of an action roguelite, human proportions, intricate metal and cloth rendered in small irregular pixel clusters, hue-shifted shadows, crisp selective outlines. More nuanced than chunky retro blocks. No blocky rectangular torso, no chibi, no cartoon vector, no blur. Detailed cel shaded pixel materials. Deep blue/purple shadows, teal oxidized armor, copper rims, bright amber eyes.
Every sprite faces RIGHT in exact 2D side view. Same subject design and scale in each row. Each humanoid feet centered at x96 y222 within its cell. Fixed scale 140px tall for row1 and row2, 175px tall for row3. Ample transparent gutters and no sprite overlaps another cell. No attack arcs painted in.
Row 1: clockwork WARDEN enemy, lean skeletal knight in battered teal-and-slate plate with ragged plum sash, angular helmet, orange visor, short rusted cleaver. Eight frames: two subtle idle frames, three distinct walking cycle frames, three attack frames (windup, forward cleave, recovery).
Row 2: ARCHER enemy, slender hunched ranger in worn indigo hood and oxidized bronze mask, tattered sage cloak, ornate copper bow. Eight frames: two idle, three walking frames, three bow attack frames (draw, loose, recover).
Row 3: BRUTE enemy, large broad armored furnace guard, bulky layered irregular rust-red pauldrons over dark plum cloth, heavy copper maul held in both hands. Anatomically articulated arms and legs, not a cube robot. Eight frames: two idle, three lumbering walking frames, three overhead smash frames (windup, strike, recover).
Row 4: mechanical MOTH enemy, one small ornate moth per cell, teal and muted-violet articulated leaflike wings with copper filigree and a glowing orange thorax. Eight-frame complete wing flap cycle with clearly different raised, lateral and lowered wing silhouettes. Each moth centered at x96 y158 in its cell, identical body size, roughly 105px wingspan. Do not mix moths with humanoids.
Priorities: exactly 8 columns and 4 rows; identical proportions within rows; readable dynamic animation poses; real alpha background.

## Regent generation

Use case: stylized-concept.
Asset type: game-ready transparent animation atlas for the Brass Regent boss in Cinderwake, a 2D side scrolling action roguelite.
One PNG 1536x1024, exactly 4 columns by 2 rows, 8 equal cells 384x512. Real transparent alpha background. No text, no grid, no labels, no scenery or contact shadows.
An ORIGINAL imposing, elegant clockwork regent: tall articulated humanoid proportions; weathered ornate brass armor, narrow glowing amber visor, asymmetrical angular crown; layered oxidized teal and deep plum cloak; glowing furnace core recessed in chest; a long heavy copper-and-glass polehammer. Do not draw a block robot. Rich anatomical armor, small irregular pixel clusters, exquisite cel-shaded pixel sprite finish. Deep indigo shadows, amber brass highlights and cool turquoise rim light. Hard color steps, no smooth airbrush. Match the premium detailed sprite look of modern action roguelites.
All 8 frames show precisely the same boss in side view facing RIGHT, about 275px tall, feet/pivot at x192 y438 within each cell. Same exact scale and consistent armor details. Every part of crown, cloak and weapon must stay within its cell with at least 45 pixels empty space horizontally. Limit wide weapon extension to achieve this. Keep the entire outline crisp, no background glows.
Row1: idle pose A; idle pose B breathing cloak motion; walking contact; walking passing.
Row2: attack anticipation crouch; polehammer raised above shoulder; powerful forward downward strike with knees bent; recover with furnace core glowing brighter.
Distinct articulated poses. No painted attack arcs or effects. Eight sprites only, exact 4x2 regular placement.
