//! Boarding weapons and armour (T21). Numbers follow Quake 2's `p_weapon.c` / `g_combat.c`
//! (damage, pellets, ammo use); distances are in metres.
// ref: REFERENCE_tobe_DELETED p_weapon.c Weapon_* damage table; g_combat.c CheckPowerArmor/ArmorSave

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weapon {
    Axe,
    Blaster,
    Scatter,
    Rattler,
    Mjolnir,
    Valkyrie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ammo {
    Shells,
    Bullets,
    Rockets,
    Slugs,
}

impl Ammo {
    pub const ALL: [Ammo; 4] = [Ammo::Shells, Ammo::Bullets, Ammo::Rockets, Ammo::Slugs];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn max(self) -> u32 {
        match self {
            Ammo::Shells => 100,
            Ammo::Bullets => 200,
            Ammo::Rockets => 50,
            Ammo::Slugs => 50,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Ammo::Shells => "SHELLS",
            Ammo::Bullets => "BULLETS",
            Ammo::Rockets => "ROCKETS",
            Ammo::Slugs => "SLUGS",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Def {
    pub name: &'static str,
    pub ammo: Option<Ammo>,
    pub cooldown: f32,
    pub damage: f32,
    pub pellets: u32,
    /// Half-angle spread in radians.
    pub spread: f32,
    pub range: f32,
    pub pierce: bool,
}

impl Weapon {
    pub const ALL: [Weapon; 6] = [
        Weapon::Axe,
        Weapon::Blaster,
        Weapon::Scatter,
        Weapon::Rattler,
        Weapon::Mjolnir,
        Weapon::Valkyrie,
    ];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn def(self) -> Def {
        let d = |name, ammo, cooldown, damage, pellets, spread, range, pierce| Def {
            name,
            ammo,
            cooldown,
            damage,
            pellets,
            spread,
            range,
            pierce,
        };
        match self {
            Weapon::Axe => d("RUNE AXE", None, 0.45, 25.0, 1, 0.0, 2.6, false),
            Weapon::Blaster => d("BLASTER", None, 0.5, 15.0, 1, 0.0, 80.0, false),
            Weapon::Scatter => d(
                "SCATTER-GUN",
                Some(Ammo::Shells),
                1.0,
                4.0,
                12,
                0.07,
                40.0,
                false,
            ),
            Weapon::Rattler => d(
                "RUNE RATTLER",
                Some(Ammo::Bullets),
                0.1,
                8.0,
                1,
                0.025,
                80.0,
                false,
            ),
            Weapon::Mjolnir => d(
                "MJOLNIR LAUNCHER",
                Some(Ammo::Rockets),
                0.8,
                120.0,
                1,
                0.0,
                120.0,
                false,
            ),
            Weapon::Valkyrie => d(
                "VALKYRIE LANCE",
                Some(Ammo::Slugs),
                1.5,
                150.0,
                1,
                0.0,
                120.0,
                true,
            ),
        }
    }
}

/// Armour tiers by protection fraction (Quake 2: jacket 30 %, combat 60 %, body 80 %).
pub fn armor_protection(value: u32) -> f32 {
    match value {
        0 => 0.0,
        1..=60 => 0.30,
        61..=120 => 0.60,
        _ => 0.80,
    }
}

/// Armour soaks `protection × damage` (rounded up) until it runs out; returns `(armor_left, hull_damage)`.
pub fn absorb(armor: u32, damage: f32) -> (u32, f32) {
    let save = (armor_protection(armor) * damage).ceil().min(armor as f32);
    (armor - save as u32, damage - save)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_absorbs_a_fraction_until_used_up() {
        assert_eq!(absorb(0, 20.0), (0, 20.0));
        let (left, hurt) = absorb(100, 20.0);
        assert_eq!(left, 88);
        assert_eq!(hurt, 8.0);
        let (left, hurt) = absorb(3, 100.0);
        assert_eq!((left, hurt), (0, 97.0));
    }

    #[test]
    fn damage_table_matches_the_design() {
        assert_eq!(Weapon::Blaster.def().damage, 15.0);
        assert_eq!(Weapon::Scatter.def().pellets, 12);
        assert_eq!(Weapon::Valkyrie.def().damage, 150.0);
        assert!(Weapon::Valkyrie.def().pierce && Weapon::Blaster.def().ammo.is_none());
    }
}
