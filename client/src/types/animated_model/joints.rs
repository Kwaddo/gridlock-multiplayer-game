use macroquad::prelude::Vec3;

use super::{AnimatedModel, Pose};

/// Body joints the game looks up by name: hitboxes are built from them and
/// the world weapon is held at the right hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointRole {
    Head,
    Chest,
    Hips,
    LeftFoot,
    RightFoot,
    LeftShoulder,
    RightShoulder,
    LeftElbow,
    RightElbow,
    LeftHand,
    RightHand,
}

pub(super) const JOINT_ROLE_COUNT: usize = 11;

/// Joint index of each role (`JointRole as usize`), resolved once per model.
pub(super) type JointRoleTable = [Option<usize>; JOINT_ROLE_COUNT];

impl JointRole {
    pub const ALL: [JointRole; JOINT_ROLE_COUNT] = [
        JointRole::Head,
        JointRole::Chest,
        JointRole::Hips,
        JointRole::LeftFoot,
        JointRole::RightFoot,
        JointRole::LeftShoulder,
        JointRole::RightShoulder,
        JointRole::LeftElbow,
        JointRole::RightElbow,
        JointRole::LeftHand,
        JointRole::RightHand,
    ];

    /// Name patterns tried in order: the first pattern any joint matches wins,
    /// and within it the first matching joint in skeleton order. A joint
    /// matches a pattern when its (lowercase) name contains every token.
    pub const fn patterns(self) -> &'static [&'static [&'static str]] {
        match self {
            JointRole::Head => &[&["head"], &["neck", "head"]],
            JointRole::Chest => &[&["spine2"], &["chest"], &["spine1"]],
            JointRole::Hips => &[&["hips"], &["pelvis"]],
            JointRole::LeftFoot => &[&["left", "foot"], &["left", "toe"]],
            JointRole::RightFoot => &[&["right", "foot"], &["right", "toe"]],
            JointRole::LeftShoulder => &[&["left", "shoulder"], &["left", "arm"]],
            JointRole::RightShoulder => &[&["right", "shoulder"], &["right", "arm"]],
            JointRole::LeftElbow => &[
                &["left", "forearm"],
                &["left", "lowerarm"],
                &["left", "elbow"],
            ],
            JointRole::RightElbow => &[
                &["right", "forearm"],
                &["right", "lowerarm"],
                &["right", "elbow"],
            ],
            JointRole::LeftHand => &[&["left", "hand"], &["left", "wrist"]],
            JointRole::RightHand => &[&["right", "hand"], &["right", "wrist"]],
        }
    }
}

/// The first joint whose name contains every token.
pub(super) fn find_joint(names: &[String], tokens: &[&str]) -> Option<usize> {
    names.iter().position(|name| tokens.iter().all(|t| name.contains(t)))
}

/// Resolve every role's joint once, so per-frame lookups need no name search.
pub(super) fn resolve_joint_roles(names: &[String]) -> JointRoleTable {
    JointRole::ALL.map(|role| {
        role.patterns()
            .iter()
            .find_map(|tokens| find_joint(names, tokens))
    })
}

impl AnimatedModel {
    // the clip's own pose, same as before
    pub fn joint_role_position(&self, role: JointRole) -> Option<Vec3> {
        self.joint_role_position_in(&self.pose, role)
    }

    // which joint is the head, the clip/skeleton knows, where it is right now, the pose knows
    // before this it could only read the clip's own pose, so hitboxes matched the shared pose
    pub fn joint_role_position_in(&self, pose: &Pose, role: JointRole) -> Option<Vec3> {
        self.joint_roles[role as usize].and_then(|i| pose.joint_positions.get(i).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name search used before indices were cached, kept as reference.
    fn search_by_name(names: &[String], positions: &[Vec3], role: JointRole) -> Option<Vec3> {
        let with_tokens = |tokens: &[&str]| {
            names.iter().zip(positions.iter()).find_map(|(name, pos)| {
                if tokens.iter().all(|t| name.contains(t)) {
                    Some(*pos)
                } else {
                    None
                }
            })
        };
        role.patterns().iter().find_map(|tokens| with_tokens(tokens))
    }

    fn cached(names: &[String], positions: &[Vec3], role: JointRole) -> Option<Vec3> {
        resolve_joint_roles(names)[role as usize].and_then(|i| positions.get(i).copied())
    }

    fn rig(names: &[&str]) -> (Vec<String>, Vec<Vec3>) {
        let names: Vec<String> = names.iter().map(|n| n.to_ascii_lowercase()).collect();
        let positions = (0..names.len()).map(|i| Vec3::splat(i as f32)).collect();
        (names, positions)
    }

    #[test]
    fn role_order_matches_discriminants() {
        for (i, role) in JointRole::ALL.iter().enumerate() {
            assert_eq!(*role as usize, i);
        }
    }

    #[test]
    fn cached_indices_match_name_search() {
        let rigs: [&[&str]; 5] = [
            // Mixamo
            &[
                "mixamorig:Hips", "mixamorig:Spine", "mixamorig:Spine1", "mixamorig:Spine2",
                "mixamorig:Neck", "mixamorig:Head", "mixamorig:HeadTop_End",
                "mixamorig:LeftShoulder", "mixamorig:LeftArm", "mixamorig:LeftForeArm",
                "mixamorig:LeftHand", "mixamorig:LeftHandIndex1", "mixamorig:RightShoulder",
                "mixamorig:RightArm", "mixamorig:RightForeArm", "mixamorig:RightHand",
                "mixamorig:RightHandThumb1", "mixamorig:LeftUpLeg", "mixamorig:LeftLeg",
                "mixamorig:LeftFoot", "mixamorig:LeftToeBase", "mixamorig:RightUpLeg",
                "mixamorig:RightLeg", "mixamorig:RightFoot", "mixamorig:RightToeBase",
            ],
            // Unreal-style: pelvis, chest via spine, lowerarm, no foot only toes
            &[
                "root", "pelvis", "spine_01", "chest", "neck_01", "head", "clavicle_l",
                "upperarm_l", "lowerarm_l", "hand_l", "left_wrist", "left_lowerarm",
                "right_shoulder", "right_lowerarm", "right_wrist", "left_toe", "right_toe",
            ],
            // Fallbacks: only later patterns match
            &["Pelvis", "Spine1", "NeckHead", "LeftElbow", "RightElbow", "LeftArm", "RightArm"],
            // Nothing matches
            &["bone_0", "bone_1", "bone_2"],
            // Empty skeleton
            &[],
        ];
        for names in rigs {
            let (names, positions) = rig(names);
            for role in JointRole::ALL {
                assert_eq!(
                    cached(&names, &positions, role),
                    search_by_name(&names, &positions, role),
                    "{role:?} in {names:?}"
                );
            }
        }
    }

    #[test]
    fn first_matching_joint_wins() {
        let (names, _) = rig(&["mixamorig:LeftHandIndex1", "mixamorig:LeftHand", "left_wrist"]);
        assert_eq!(resolve_joint_roles(&names)[JointRole::LeftHand as usize], Some(0));
        let (names, _) = rig(&["left_wrist", "mixamorig:LeftHand"]);
        assert_eq!(resolve_joint_roles(&names)[JointRole::LeftHand as usize], Some(1));
    }
}
