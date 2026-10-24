<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# OwlbearBuilder

OwlbearBuilder bridges Pathbuilder 2e character exports and Pathfinder Second Edition (Remaster) creature stat blocks into Owlbear Rodeo 2.0. It gives your table a shared rules popover without requiring players to install software or expose ports across the internet.

The entire project compiles into a single Rust binary built strictly on the Rust standard library (`std`), with zero external crate dependencies and no JavaScript bundler.

## Features

- **Party Overview:** View character AC, Hit Points, temporary HP, Hero Points, Perception, saving throws, skills, Lores, strikes, and spellbooks in one place.
- **Character Importing and Level-Up Merging:** Import characters from Pathbuilder 2e ("Export JSON"). Re-importing a character after leveling up preserves their current HP, active conditions, and linked map token while reporting what changed. (Wanderer's Guide exports are not supported yet.)
- **Creatures and Elite/Weak Variants:** Paste creature stat block text copied directly from Archives of Nethys, or spawn any of the six built-in *Pathfinder Monster Core* creatures (Goblin Warrior, Orc Brute, Skeleton Guard, Kobold Warrior, Giant Rat, and Ogre Warrior). Apply Normal, Elite, or Weak adjustments on import or on the fly.
- **Token Binding:** Bind characters and creature instances to tokens on the Owlbear Rodeo scene so HP and status updates stay synchronized with the tabletop.
- **Initiative Tracker:** Track combat order using Pathfinder 2e tie-breaking rules, delay turns, and automatically tick timed conditions as turns advance.
- **Rolls and Degrees of Success:** Roll Perception, saves, skill checks, strikes with Multiple Attack Penalty (MAP), damage (normal and critical), and spell attacks. Checks against a DC or target AC automatically evaluate the four degrees of success (critical success, success, failure, critical failure), including natural 20 and natural 1 step adjustments.
- **Assurance and Secret Checks:** Use Assurance on trained skills to take `10 + proficiency bonus` without rolling dice, or roll secret checks visible only to the GM (individually or across the whole party).
- **Spell Area Templates:** Place bursts, cones, emanations, and lines on the Owlbear Rodeo canvas computed for Pathfinder 2e's 5-10-5 diagonal grid rules.

## How a Table Runs It (GM Relay)

OwlbearBuilder uses a GM relay architecture so that only the GM runs the local rules server, while players connect solely through Owlbear Rodeo in their browsers:

1. **Host the static extension files:** Export the static web assets and host them on any HTTPS static host (such as GitHub Pages, Netlify, or a personal web server):
   ```bash
   owlbearbuilder export-static site/
   ```
2. **Install the extension in Owlbear Rodeo:** In Owlbear Rodeo 2.0, add the hosted `https://<host>/manifest.json` URL as a custom extension.
3. **Run the server on the GM's machine:** When it is time to play, the GM starts the local server on their own computer:
   ```bash
   owlbearbuilder serve --state-file campaign.json
   ```
   By default, `owlbearbuilder serve` listens on `127.0.0.1:8080`, and `--state-file` saves the room state across sessions.
4. **Play together:** Only the GM's browser talks to `127.0.0.1:8080`. Players need nothing installed on their machines: when a player clicks an action in their Owlbear Rodeo popover, the request relays to the GM's popover over Owlbear Rodeo broadcasts, executes against the GM's local server, and broadcasts the updated room state back to the table.

### Quick Solo Test

If you are testing on a single computer and do not need remote players to load the extension over HTTPS, `owlbearbuilder serve` also serves the extension assets directly. Run `owlbearbuilder serve` and add `http://127.0.0.1:8080/manifest.json` as an extension in Owlbear Rodeo on the same machine.

## Command-Line Usage

In addition to running the extension server, the `owlbearbuilder` binary provides command-line utilities for inspecting exports, parsing creature stat blocks, rolling dice, and exporting static assets:

```text
Usage: owlbearbuilder <command>
Commands:
  serve [--bind ADDR] [--state-file FILE] [--seed N]
  inspect <file>
  parse-monster <file> [--elite|--weak] [--json]
  roll <expr> [--dc N] [--seed N]
  export-static <dir>
```

### Examples

Start the local GM server on the default address (`127.0.0.1:8080`) and persist room state to disk:

```bash
owlbearbuilder serve --state-file room.json
```

Print a summary of a Pathbuilder 2e JSON export (level, HP, AC, saves, strikes with MAP, skills, Lores, and known spells):

```bash
owlbearbuilder inspect fighter.json
```

Parse an Archives of Nethys plain-text creature stat block with the Elite adjustment applied, outputting either a human-readable summary or JSON:

```bash
owlbearbuilder parse-monster goblin.txt --elite
owlbearbuilder parse-monster goblin.txt --weak --json
```

Roll a dice expression from the terminal, optionally evaluating the result against a Difficulty Class (DC) with a deterministic RNG seed:

```bash
owlbearbuilder roll 1d20+9 --dc 20
owlbearbuilder roll 2d6+4 --seed 42
```

Write the static Owlbear Rodeo extension files (`manifest.json`, `index.html`, `obr_bridge.js`, and `icon.svg`) to a directory for static hosting:

```bash
owlbearbuilder export-static site/
```

## Building and Testing

OwlbearBuilder requires only a standard Rust toolchain. To keep builds polite on shared machines, constrain Cargo concurrency to 2 jobs:

```bash
cargo build --release -j 2
```

The compiled binary will be located at `target/release/owlbearbuilder`.

To run the full workspace test suite:

```bash
cargo test -j 2 --workspace
```

See `CONTRIBUTING.md` for coding conventions, architecture constraints, and rules citation guidelines.

## Licences and Notices

### Software Licence

OwlbearBuilder's source code is licensed under the **GNU General Public License v3.0 or later** (`GPL-3.0-or-later`). See `LICENSE` for the full text.

### Open RPG Creative (ORC) License Notice

Game rules data embedded in this project (common spells in `obb-core/src/spell_data.rs`, sample creatures in `obb-core/src/bestiary_data.rs`, and weapon trait mechanics) comes from *Pathfinder Player Core*, *Pathfinder GM Core*, and *Pathfinder Monster Core* and is used under the **ORC License**.

- **ORC Notice:** This product is licensed under the ORC License held in the Library of Congress at TX 9-307-067 and available online at various locations including https://paizo.com/orclicense. All warranties are disclaimed as set forth therein.
- **Attribution:** This product is based on the following Licensed Material:
  - *Pathfinder Player Core* © 2023 Paizo Inc., Designed by Logan Bonner, Jason Bulmahn, Stephen Radney-MacFarland, and Mark Seifter.
  - *Pathfinder GM Core* © 2023 Paizo Inc., Designed by Logan Bonner and Mark Seifter.
  - *Pathfinder Monster Core* © 2024 Paizo Inc., Designed by Logan Bonner, Jason Bulmahn, Stephen Radney-MacFarland, and Mark Seifter.
- **Reserved Material:** none (no Reserved Material is used).
- **Expressly Designated Licensed Material:** none.

### Trademark and Independence Disclaimer

OwlbearBuilder is an independent project and is not published, endorsed, or specifically approved by Paizo Inc. or by Owlbear Rodeo. Pathfinder is a registered trademark of Paizo Inc.
