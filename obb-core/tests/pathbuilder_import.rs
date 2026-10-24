// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::character::{Ability, ProfRank, Skill};
use obb_core::import_pb::{import_pathbuilder_json, strike_from_pathbuilder};
use obb_core::json::JsonValue;
use obb_core::weapons::weapon_info;

const FIGHTER: &str = r#"{"success":true,"build":{
  "name":"Amiri Stormborn","class":"Fighter","dualClass":null,"level":5,
  "ancestry":"Human","heritage":"Versatile Human","background":"Warrior",
  "alignment":"N","gender":"Female","age":"27","deity":"Gorum","size":2,"sizeName":"Medium",
  "keyability":"str","languages":["Common","Hallit"],
  "attributes":{"ancestryhp":8,"classhp":10,"bonushp":0,"bonushpPerLevel":0,"speed":25,"speedBonus":5},
  "abilities":{"str":18,"dex":14,"con":14,"int":10,"wis":12,"cha":10,"breakdown":{}},
  "proficiencies":{"classDC":2,"perception":4,"fortitude":4,"reflex":4,"will":2,
    "heavy":2,"medium":2,"light":2,"unarmored":2,"advanced":2,"martial":6,"simple":6,"unarmed":6,
    "castingArcane":0,"castingDivine":0,"castingOccult":0,"castingPrimal":0,
    "acrobatics":2,"arcana":0,"athletics":4,"crafting":0,"deception":0,"diplomacy":0,
    "intimidation":4,"medicine":0,"nature":0,"occultism":0,"performance":0,"religion":0,
    "society":0,"stealth":0,"survival":2,"thievery":0},
  "mods":{"Athletics":{"Item Bonus":1,"Status Bonus":1},"Warfare Lore":{"Item Bonus":1},"Nonsense":{"Item Bonus":3}},
  "feats":[["Power Attack",null,"Class Feat",1],["Untrained Improvisation",null,"General Feat",3],["Shield Block",null,"General Feat",1]],
  "specials":["Attack of Opportunity","Shield Block","Bravery"],
  "lores":[["Warfare",2]],
  "equipment":[["Healing Potion (Minor)",2,"Invested"]],
  "weapons":[
    {"name":"Longsword","qty":1,"prof":"martial","die":"d8","pot":1,"str":"striking","mat":null,"display":"+1 Striking Longsword","runes":[],"damageType":"S","attack":16,"damageBonus":4,"extraDamage":["1d6 fire"],"increasedDice":false,"isInventor":false},
    {"name":"Shortsword","qty":1,"prof":"martial","die":"d6","pot":0,"str":"","mat":null,"display":"Shortsword","runes":[],"damageType":"P","attack":15,"damageBonus":4,"extraDamage":[],"increasedDice":false,"isInventor":false},
    {"name":"Composite Longbow","qty":1,"prof":"martial","die":"d8","pot":0,"str":"","mat":null,"display":"Composite Longbow","runes":[],"damageType":"P","attack":13,"damageBonus":2,"extraDamage":[],"increasedDice":false,"isInventor":false}
  ],
  "armor":[{"name":"Full Plate","qty":1,"prof":"heavy","pot":1,"res":"resilient","mat":null,"display":"+1 Resilient Full Plate","worn":true,"runes":[]}],
  "spellCasters":[],"focusPoints":0,"focus":{},
  "acTotal":{"acProfBonus":7,"acAbilityBonus":0,"acItemBonus":7,"acTotal":24,"shieldBonus":"2"},
  "pets":[],"familiars":[]}}"#;

const WIZARD: &str = r#"{"name":"Ezren","class":"Wizard","level":5,"ancestry":"Human","heritage":"Skilled Human",
  "background":"Scholar","sizeName":"Medium","keyability":"int",
  "attributes":{"ancestryhp":8,"classhp":6,"bonushp":0,"bonushpPerLevel":0,"speed":25,"speedBonus":0},
  "abilities":{"str":0,"dex":2,"con":1,"int":4,"wis":1,"cha":0},
  "proficiencies":{"classDC":2,"perception":2,"fortitude":2,"reflex":2,"will":4,"arcana":4,"society":2},
  "lores":[],"feats":[],"specials":[],"weapons":[],"armor":[],
  "spellCasters":[{"name":"Wizard","magicTradition":"arcane","spellcastingType":"prepared","ability":"int",
    "proficiency":2,"focusPoints":0,"innate":false,"perDay":[5,3,3,2,0,0,0,0,0,0,0],
    "spells":[{"spellLevel":0,"list":["Electric Arc","Shield","Detect Magic"]},
              {"spellLevel":1,"list":["Force Barrage","Grease"]},
              {"spellLevel":3,"list":["Fireball"]}]},
    {"name":"Innate Spells","magicTradition":"arcane","spellcastingType":"spontaneous","ability":"cha",
    "proficiency":2,"focusPoints":0,"innate":true,"spells":[{"spellLevel":0,"list":["Prestidigitation"]}]}],
  "focusPoints":1,
  "focus":{"arcane":{"int":{"abilityBonus":4,"proficiency":2,"itemBonus":0,"focusCantrips":[],"focusSpells":["Force Bolt"]}}},
  "acTotal":{"acProfBonus":7,"acAbilityBonus":2,"acItemBonus":0,"acTotal":19,"shieldBonus":null}}"#;

#[test]
fn imports_identity_abilities_and_hp() {
    let c = import_pathbuilder_json(FIGHTER).unwrap();
    assert_eq!(c.id, "amiri-stormborn");
    assert_eq!(c.name, "Amiri Stormborn");
    assert_eq!((c.class.as_str(), c.level), ("Fighter", 5));
    assert_eq!(
        (
            c.ancestry.as_str(),
            c.heritage.as_str(),
            c.background.as_str()
        ),
        ("Human", "Versatile Human", "Warrior")
    );
    assert_eq!(c.size, "Medium");
    assert_eq!(c.abilities, [4, 2, 2, 0, 1, 0]);
    assert_eq!(c.key_ability, Ability::Str);
    assert_eq!(c.max_hp, 8 + 5 * (10 + 2));
    assert_eq!(c.current_hp, c.max_hp);
    assert_eq!(c.speed, 30);
    assert_eq!(c.ac_base, 24);
    assert_eq!(c.shield_bonus, 2);
    assert_eq!(c.player_name, "");
}

#[test]
fn imports_proficiencies_lores_and_item_bonuses() {
    let c = import_pathbuilder_json(FIGHTER).unwrap();
    assert_eq!(c.perception_rank, ProfRank::Expert);
    assert_eq!(
        (c.fort_rank, c.reflex_rank, c.will_rank),
        (ProfRank::Expert, ProfRank::Expert, ProfRank::Trained)
    );
    assert_eq!(c.class_dc_rank, ProfRank::Trained);
    assert_eq!(c.skill_ranks[Skill::Athletics as usize], ProfRank::Expert);
    assert_eq!(c.skill_ranks[Skill::Survival as usize], ProfRank::Trained);
    assert_eq!(c.skill_ranks[Skill::Thievery as usize], ProfRank::Untrained);
    assert_eq!(c.lores.len(), 1);
    assert_eq!(
        (c.lores[0].name.as_str(), c.lores[0].rank),
        ("Warfare", ProfRank::Trained)
    );
    assert_eq!(c.item_bonuses.get("athletics"), Some(&1));
    assert_eq!(c.item_bonuses.get("warfare lore"), Some(&1));
    for save in ["fortitude", "reflex", "will"] {
        assert_eq!(c.item_bonuses.get(save), Some(&1), "{save}");
    }
    assert_eq!(c.item_bonuses.len(), 5);
}

#[test]
fn imports_feats_and_specials_without_duplicates() {
    let c = import_pathbuilder_json(FIGHTER).unwrap();
    assert_eq!(
        c.feats,
        [
            "Power Attack",
            "Untrained Improvisation",
            "Shield Block",
            "Attack of Opportunity",
            "Bravery"
        ]
    );
}

#[test]
fn imports_strikes() {
    let c = import_pathbuilder_json(FIGHTER).unwrap();
    assert_eq!(c.strikes.len(), 3);
    let sword = &c.strikes[0];
    assert_eq!(sword.name, "+1 Striking Longsword");
    assert_eq!(sword.attack_bonus, 16);
    assert_eq!(sword.attack_ability, Ability::Str);
    assert!(!sword.is_ranged);
    assert_eq!(
        (
            sword.damage.dice_count,
            sword.damage.die_sides,
            sword.damage.flat_bonus
        ),
        (2, 8, 4)
    );
    assert_eq!(sword.damage_type, "slashing");
    assert_eq!(sword.extra_damage, ["1d6 fire"]);
    assert!(sword.traits.contains(&"versatile p".to_string()));

    // Finesse, but Str +4 beats Dex +2.
    let short = &c.strikes[1];
    assert!(short.is_agile());
    assert!(short.traits.contains(&"finesse".to_string()));
    assert_eq!(short.attack_ability, Ability::Str);
    assert_eq!(short.damage_type, "piercing");

    let bow = &c.strikes[2];
    assert!(bow.is_ranged);
    assert_eq!(bow.attack_ability, Ability::Dex);
    assert_eq!(bow.damage.deadly, Some(10));
    assert_eq!(bow.damage.fatal, None);
    assert!(bow.traits.contains(&"propulsive".to_string()));
}

#[test]
fn strike_details() {
    let rogue = [0, 4, 1, 1, 2, 1];
    let w = |s: &str| JsonValue::parse(s).unwrap();
    let rapier = strike_from_pathbuilder(&w(r#"{"name":"Rapier","display":"Rapier","die":"d6","str":"greaterStriking","attack":12,"damageBonus":0,"damageType":"P"}"#), &rogue).unwrap();
    assert_eq!(rapier.attack_ability, Ability::Dex);
    assert_eq!(
        (rapier.damage.dice_count, rapier.damage.deadly),
        (3, Some(8))
    );
    let pick = strike_from_pathbuilder(&w(r#"{"name":"Pick","die":"d6","str":"majorStriking","increasedDice":true,"attack":9,"damageBonus":1,"damageType":"P"}"#), &rogue).unwrap();
    assert_eq!(pick.name, "Pick");
    assert_eq!(
        (
            pick.damage.dice_count,
            pick.damage.die_sides,
            pick.damage.fatal
        ),
        (4, 8, Some(10))
    );
    let odd = strike_from_pathbuilder(&w(r#"{"name":"Mystery Blade","die":"d10","str":"","attack":5,"damageBonus":3,"damageType":"cold"}"#), &rogue).unwrap();
    assert!(!odd.is_ranged);
    assert!(odd.traits.is_empty());
    assert_eq!(odd.attack_ability, Ability::Str);
    assert_eq!(odd.damage_type, "cold");
    let custom = strike_from_pathbuilder(&w(r#"{"name":"Claw","die":"d4","str":"","attack":7,"damageBonus":2,"damageType":"S","traits":["Agile","Finesse","Unarmed"]}"#), &rogue).unwrap();
    assert!(custom.is_agile());
    assert_eq!(custom.attack_ability, Ability::Dex);
    assert!(strike_from_pathbuilder(&w(r#"{"name":"Broken","die":"dx"}"#), &rogue).is_err());
}

#[test]
fn imports_spellcasting_and_focus() {
    let c = import_pathbuilder_json(WIZARD).unwrap();
    assert_eq!(c.abilities, [0, 2, 1, 4, 1, 0]);
    assert_eq!(c.max_hp, 8 + 5 * (6 + 1));
    assert_eq!(c.shield_bonus, 0);
    assert_eq!(c.skill_ranks[Skill::Arcana as usize], ProfRank::Expert);
    assert_eq!(
        c.skill_ranks[Skill::Athletics as usize],
        ProfRank::Untrained
    );
    assert_eq!(c.spellcasting.len(), 3);
    let wiz = &c.spellcasting[0];
    assert_eq!(
        (wiz.name.as_str(), wiz.tradition.as_str(), wiz.kind.as_str()),
        ("Wizard", "arcane", "prepared")
    );
    assert_eq!((wiz.ability, wiz.rank), (Ability::Int, ProfRank::Trained));
    assert_eq!(
        wiz.spells[0],
        (
            0,
            vec![
                "Electric Arc".to_string(),
                "Shield".into(),
                "Detect Magic".into()
            ]
        )
    );
    assert_eq!(wiz.spells[2], (3, vec!["Fireball".to_string()]));
    assert_eq!(c.spellcasting[1].kind, "innate");
    assert_eq!(c.spellcasting[1].ability, Ability::Cha);
    let focus = &c.spellcasting[2];
    assert_eq!(
        (
            focus.name.as_str(),
            focus.kind.as_str(),
            focus.tradition.as_str()
        ),
        ("Focus Spells", "focus", "arcane")
    );
    assert_eq!(
        (focus.ability, focus.rank, focus.focus_points),
        (Ability::Int, ProfRank::Trained, 1)
    );
    assert_eq!(focus.spells, vec![(3, vec!["Force Bolt".to_string()])]);
}

#[test]
fn rejects_bad_exports() {
    assert!(import_pathbuilder_json("not json").is_err());
    assert!(import_pathbuilder_json(r#"{"success": false}"#).is_err());
    assert!(import_pathbuilder_json(r#"{"success": true, "build": {"level": 3}}"#).is_err());
    assert!(
        import_pathbuilder_json(r#"{"success": true, "build": {"name": "No Level"}}"#).is_err()
    );
    let minimal = import_pathbuilder_json(r#"{"build": {"name": "Seelah", "level": 1}}"#).unwrap();
    assert_eq!(minimal.size, "Medium");
    assert!(minimal.strikes.is_empty());
}

#[test]
fn weapon_table() {
    let traits = |n: &str| {
        weapon_info(n)
            .unwrap_or_else(|| panic!("{n} missing"))
            .traits
            .to_vec()
    };
    assert!(traits("Dagger").contains(&"agile"));
    assert!(traits("Dagger").contains(&"thrown 10 ft"));
    assert!(traits("rapier").contains(&"deadly d8"));
    assert!(traits("Greatpick").contains(&"fatal d12"));
    assert!(traits("Longsword (cold iron)").contains(&"versatile p"));
    assert_eq!(weapon_info("Longbow").unwrap().range_ft, Some(100));
    assert_eq!(weapon_info("Javelin").unwrap().range_ft, Some(30));
    assert_eq!(weapon_info("Spear").unwrap().range_ft, None);
    assert!(weapon_info("Spoon of Doom").is_none());
}
