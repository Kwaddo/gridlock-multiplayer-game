use macroquad::prelude::*;

use crate::types::player::PlayerSkin;
use crate::weapons::combat::HitZone;

/*
    a chooser like ue5's chooser plugin: a table of rows, each row is some conditions + what to play
    rows are checked top to bottom, the first row where every condition matches wins
    a result can be one clip, a weighted random pick, or another table (nested, like ue)
    used for deaths now, flinches later
    before this the death was picked by hand in 3 different places with stance only
*/

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    Standing,
    Crouching,
    Prone,
}

// everything a choice can look at, filled in once by whoever asks
#[derive(Clone, Copy, Debug)]
pub struct ChooserContext {
    pub stance: Stance,
    pub hit_angle: Option<f32>, // where the hit came from in degrees, 0 = in front, +-180 = behind, none = unknown
    pub headshot: bool,
    pub hit_zone: Option<HitZone>, // for flinches later
    pub seed: u32, // same seed on every client = same random pick everywhere
}

// one condition on the context
pub enum Column {
    Stance(Stance),
    AngleBetween(f32, f32), // hit_angle inside this range, a range like (120, -120) wraps round the back
    AngleUnknown,
    Headshot(bool),
    Zone(HitZone),
}

// what a matching row gives back
pub enum Choice {
    Clip(PlayerSkin),
    Random(&'static [(PlayerSkin, f32)]), // weights, picked with ctx.seed
    Table(&'static ChooserTable),         // hand off to another table
}

pub struct Row {
    pub when: &'static [Column],
    pub then: Choice,
}

pub struct ChooserTable {
    pub rows: &'static [Row],
    pub fallback: PlayerSkin, // nothing matched
}

impl ChooserTable {
    // first row where every column matches, else the fallback
    pub fn choose(&self, ctx: &ChooserContext) -> PlayerSkin {
        for row in self.rows {
            if row.when.iter().all(|c| c.matches(ctx)) {
                return row.then.resolve(ctx);
            }
        }
        self.fallback
    }
}

impl Column {
    fn matches(&self, ctx: &ChooserContext) -> bool {
        match *self {
            Column::Stance(s) => ctx.stance == s,
            Column::AngleUnknown => ctx.hit_angle.is_none(),
            Column::AngleBetween(from, to) => ctx.hit_angle.is_some_and(|a| angle_between(a, from, to)),
            Column::Headshot(h) => ctx.headshot == h,
            Column::Zone(z) => ctx.hit_zone == Some(z),
        }
    }
}

impl Choice {
    fn resolve(&self, ctx: &ChooserContext) -> PlayerSkin {
        match self {
            Choice::Clip(skin) => *skin,
            Choice::Table(table) => table.choose(ctx),
            Choice::Random(options) => pick_weighted(options, ctx.seed),
        }
    }
}

// is `a` inside the range going clockwise from `from` to `to`, wrapping at +-180
fn angle_between(a: f32, from: f32, to: f32) -> bool {
    if from <= to { a >= from && a <= to } else { a >= from || a <= to }
}

// weighted pick from a seed, same seed = same pick on every client
fn pick_weighted(options: &[(PlayerSkin, f32)], seed: u32) -> PlayerSkin {
    let total: f32 = options.iter().map(|(_, w)| w).sum();
    let roll = (seed.wrapping_mul(2654435761) >> 8) as f32 / (1u32 << 24) as f32 * total; // 0..total
    let mut acc = 0.0;
    for (skin, w) in options {
        acc += w;
        if roll < acc {
            return *skin;
        }
    }
    options.last().map(|o| o.0).unwrap_or(PlayerSkin::DeathFront)
}

// where the killer is from the victim, 0 front, 90 right, -90 left, 180 behind
pub fn hit_angle(victim_pos: Vec3, victim_yaw: f32, killer_pos: Option<Vec3>) -> Option<f32> {
    let to_killer = killer_pos? - victim_pos;
    let flat = vec2(to_killer.x, to_killer.z);
    if flat.length() < 0.1 {
        return None;
    }
    let facing = vec2(victim_yaw.cos(), victim_yaw.sin());
    let right = vec2(-victim_yaw.sin(), victim_yaw.cos());
    Some(flat.dot(right).atan2(flat.dot(facing)).to_degrees())
}

/*
    standing deaths by hit direction, crouch and prone get one each
    sides fall through to the fallback, right clip covers left
*/
pub static DEATH_TABLE: ChooserTable = ChooserTable {
    rows: &[
        Row { when: &[Column::Stance(Stance::Crouching)], then: Choice::Clip(PlayerSkin::CrouchDeath) },
        Row { when: &[Column::Stance(Stance::Prone)], then: Choice::Clip(PlayerSkin::ProneDeath) },
        Row { when: &[Column::AngleUnknown], then: Choice::Clip(PlayerSkin::DeathFront) },
        Row {
            when: &[Column::Headshot(true), Column::AngleBetween(-60.0, 60.0)],
            then: Choice::Clip(PlayerSkin::DeathFrontHeadshot),
        },
        Row {
            when: &[Column::Headshot(true), Column::AngleBetween(120.0, -120.0)],
            then: Choice::Clip(PlayerSkin::DeathBackHeadshot),
        },
        Row { when: &[Column::AngleBetween(-60.0, 60.0)], then: Choice::Clip(PlayerSkin::DeathFront) },
        Row { when: &[Column::AngleBetween(120.0, -120.0)], then: Choice::Clip(PlayerSkin::DeathBack) },
    ],
    fallback: PlayerSkin::DeathSide,
};

pub fn stance(prone: bool, crouching: bool) -> Stance {
    if prone {
        Stance::Prone
    } else if crouching {
        Stance::Crouching
    } else {
        Stance::Standing
    }
}

// the one call every death spot uses
pub fn choose_death(
    stance: Stance,
    victim_pos: Vec3,
    victim_yaw: f32,
    killer_pos: Option<Vec3>,
    headshot: bool,
    seed: u32,
) -> PlayerSkin {
    let hit_angle = hit_angle(victim_pos, victim_yaw, killer_pos);
    DEATH_TABLE.choose(&ChooserContext { stance, hit_angle, headshot, hit_zone: None, seed })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(stance: Stance, hit_angle: Option<f32>, headshot: bool) -> ChooserContext {
        ChooserContext { stance, hit_angle, headshot, hit_zone: None, seed: 0 }
    }

    // a small table just for testing: crouch first, then a front headshot, then front, else fallback
    static TEST_TABLE: ChooserTable = ChooserTable {
        rows: &[
            Row { when: &[Column::Stance(Stance::Crouching)], then: Choice::Clip(PlayerSkin::CrouchDeath) },
            Row {
                when: &[Column::Headshot(true), Column::AngleBetween(-60.0, 60.0)],
                then: Choice::Clip(PlayerSkin::DeathFrontHeadshot),
            },
            Row { when: &[Column::AngleBetween(-60.0, 60.0)], then: Choice::Clip(PlayerSkin::DeathFront) },
        ],
        fallback: PlayerSkin::DeathSide,
    };

    #[test]
    fn first_matching_row_wins() {
        // crouching matches row 1 even though it's also a front headshot
        assert_eq!(TEST_TABLE.choose(&ctx(Stance::Crouching, Some(0.0), true)), PlayerSkin::CrouchDeath);
        // standing front headshot matches row 2 before the plain front row
        assert_eq!(TEST_TABLE.choose(&ctx(Stance::Standing, Some(10.0), true)), PlayerSkin::DeathFrontHeadshot);
        assert_eq!(TEST_TABLE.choose(&ctx(Stance::Standing, Some(10.0), false)), PlayerSkin::DeathFront);
    }

    #[test]
    fn nothing_matches_gives_the_fallback() {
        assert_eq!(TEST_TABLE.choose(&ctx(Stance::Standing, Some(90.0), false)), PlayerSkin::DeathSide);
        assert_eq!(TEST_TABLE.choose(&ctx(Stance::Standing, None, false)), PlayerSkin::DeathSide);
    }

    #[test]
    fn angle_ranges_wrap_round_the_back() {
        // (120, -120) = everything behind, crossing +-180
        assert!(angle_between(180.0, 120.0, -120.0));
        assert!(angle_between(-170.0, 120.0, -120.0));
        assert!(angle_between(130.0, 120.0, -120.0));
        assert!(!angle_between(0.0, 120.0, -120.0));
        assert!(!angle_between(90.0, 120.0, -120.0));
        // a normal range doesnt wrap
        assert!(angle_between(0.0, -60.0, 60.0));
        assert!(!angle_between(90.0, -60.0, 60.0));
    }

    #[test]
    fn same_seed_same_pick_and_seeds_spread_out() {
        static OPTIONS: [(PlayerSkin, f32); 2] = [(PlayerSkin::DeathFront, 1.0), (PlayerSkin::DeathBack, 1.0)];
        // every client with the same seed agrees
        assert_eq!(pick_weighted(&OPTIONS, 42), pick_weighted(&OPTIONS, 42));
        // over many seeds both options come up, roughly evenly
        let fronts = (0..1000u32).filter(|&s| pick_weighted(&OPTIONS, s) == PlayerSkin::DeathFront).count();
        assert!((350..650).contains(&fronts), "{fronts}");
        // a zero weight option never comes up
        static ONE_SIDED: [(PlayerSkin, f32); 2] = [(PlayerSkin::DeathFront, 1.0), (PlayerSkin::DeathBack, 0.0)];
        assert!((0..200u32).all(|s| pick_weighted(&ONE_SIDED, s) == PlayerSkin::DeathFront));
    }

    #[test]
    fn nested_tables_resolve_through_the_parent() {
        static INNER: ChooserTable = ChooserTable {
            rows: &[Row { when: &[Column::Headshot(true)], then: Choice::Clip(PlayerSkin::DeathBackHeadshot) }],
            fallback: PlayerSkin::DeathBack,
        };
        static OUTER: ChooserTable = ChooserTable {
            rows: &[Row { when: &[Column::AngleBetween(120.0, -120.0)], then: Choice::Table(&INNER) }],
            fallback: PlayerSkin::DeathFront,
        };
        assert_eq!(OUTER.choose(&ctx(Stance::Standing, Some(180.0), true)), PlayerSkin::DeathBackHeadshot);
        assert_eq!(OUTER.choose(&ctx(Stance::Standing, Some(180.0), false)), PlayerSkin::DeathBack);
        assert_eq!(OUTER.choose(&ctx(Stance::Standing, Some(0.0), true)), PlayerSkin::DeathFront);
    }

    fn death(stance: Stance, angle: Option<f32>, headshot: bool) -> PlayerSkin {
        DEATH_TABLE.choose(&ctx(stance, angle, headshot))
    }

    #[test]
    fn hit_angle_points_at_the_killer() {
        let near = |a: Option<f32>, b: f32| a.is_some_and(|a| (a - b).abs() < 0.5 || (a.abs() - 180.0).abs() < 0.5 && b.abs() == 180.0);
        // facing +x
        assert!(near(hit_angle(Vec3::ZERO, 0.0, Some(vec3(5.0, 0.0, 0.0))), 0.0));
        assert!(near(hit_angle(Vec3::ZERO, 0.0, Some(vec3(-5.0, 0.0, 0.0))), 180.0));
        assert!(near(hit_angle(Vec3::ZERO, 0.0, Some(vec3(0.0, 0.0, 5.0))), 90.0));
        assert!(near(hit_angle(Vec3::ZERO, 0.0, Some(vec3(0.0, 0.0, -5.0))), -90.0));
        // turned 90 degrees, the same killer is now in front
        assert!(near(hit_angle(Vec3::ZERO, std::f32::consts::FRAC_PI_2, Some(vec3(0.0, 0.0, 5.0))), 0.0));
        // height doesnt matter
        assert!(near(hit_angle(Vec3::ZERO, 0.0, Some(vec3(5.0, 3.0, 0.0))), 0.0));
        assert_eq!(hit_angle(Vec3::ZERO, 0.0, None), None);
        assert_eq!(hit_angle(Vec3::ZERO, 0.0, Some(vec3(0.05, 0.0, 0.0))), None);
    }

    #[test]
    fn death_table_rows() {
        use Stance::*;
        assert_eq!(death(Crouching, Some(180.0), true), PlayerSkin::CrouchDeath);
        assert_eq!(death(Prone, Some(0.0), false), PlayerSkin::ProneDeath);
        assert_eq!(death(Standing, None, true), PlayerSkin::DeathFront);
        assert_eq!(death(Standing, Some(10.0), true), PlayerSkin::DeathFrontHeadshot);
        assert_eq!(death(Standing, Some(-170.0), true), PlayerSkin::DeathBackHeadshot);
        assert_eq!(death(Standing, Some(-40.0), false), PlayerSkin::DeathFront);
        assert_eq!(death(Standing, Some(150.0), false), PlayerSkin::DeathBack);
        assert_eq!(death(Standing, Some(90.0), false), PlayerSkin::DeathSide);
        assert_eq!(death(Standing, Some(-90.0), false), PlayerSkin::DeathSide);
        assert_eq!(death(Standing, Some(90.0), true), PlayerSkin::DeathSide);
    }

    #[test]
    fn choose_death_from_positions() {
        let me = Vec3::ZERO;
        // facing +x, shot from in front, from behind, from the side
        assert_eq!(choose_death(Stance::Standing, me, 0.0, Some(vec3(5.0, 0.0, 0.0)), false, 1), PlayerSkin::DeathFront);
        assert_eq!(choose_death(Stance::Standing, me, 0.0, Some(vec3(-5.0, 0.0, 0.0)), true, 1), PlayerSkin::DeathBackHeadshot);
        assert_eq!(choose_death(Stance::Standing, me, 0.0, Some(vec3(0.0, 0.0, 5.0)), false, 1), PlayerSkin::DeathSide);
        assert_eq!(choose_death(stance(false, true), me, 0.0, None, true, 1), PlayerSkin::CrouchDeath);
        assert_eq!(choose_death(stance(true, true), me, 0.0, None, false, 1), PlayerSkin::ProneDeath);
        assert_eq!(choose_death(stance(false, false), me, 0.0, None, false, 1), PlayerSkin::DeathFront);
    }
}
