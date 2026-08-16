use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 10-level pool of generic skill rank titles (Chinese idioms).
/// For a skill with `max_level = N`, we pick N evenly-spaced entries.
const DEFAULT_TITLE_POOL: &[&str] = &[
    "初窥门径", // 1  – first glimpse
    "初学乍练", // 2  – just started
    "略知一二", // 3  – know a bit
    "小有所成", // 4  – some achievement
    "驾轻就熟", // 5  – handling with ease
    "融会贯通", // 6  – thoroughly understood
    "得心应手", // 7  – at one's fingertips
    "炉火纯青", // 8  – perfected
    "出神入化", // 9  – transcendent
    "登峰造极", // 10 – pinnacle
];

/// Return default level titles for a skill with the given `max_level`.
///
/// * `max_level == 0` → empty vec
/// * `max_level == 1` → last title only (reaching the sole level = mastery)
/// * `max_level <= 10` → pick `max_level` evenly-spaced entries from the pool
/// * `max_level > 10` → all 10 pool entries + generic "Lv.N" for the rest
pub fn default_level_titles(max_level: u32) -> Vec<String> {
    let n = max_level as usize;
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![DEFAULT_TITLE_POOL.last().unwrap().to_string()];
    }
    let pool_len = DEFAULT_TITLE_POOL.len(); // 10
    if n <= pool_len {
        let last = pool_len - 1;
        (0..n)
            .map(|i| DEFAULT_TITLE_POOL[i * last / (n - 1)].to_string())
            .collect()
    } else {
        let mut titles: Vec<String> = DEFAULT_TITLE_POOL.iter().map(|s| s.to_string()).collect();
        for i in pool_len..n {
            titles.push(format!("Lv.{}", i + 1));
        }
        titles
    }
}

// --- Deserialization structs (from JSON files) ---

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SkillFile {
    pub version: u32,
    pub skills: Vec<SkillDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub max_level: u32,
    #[serde(default)]
    pub level_titles: Vec<String>,
    pub level_thresholds: Vec<LevelThreshold>,
    pub nodes: Vec<SkillNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_image: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillNode {
    pub node_id: String,
    pub achievement_id: String,
    pub points: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LevelThreshold {
    pub level: u32,
    pub points_required: u32,
    #[serde(default)]
    pub required_key_achievements: Vec<String>,
}

// --- Response structs (sent to frontend) ---

/// Result of evaluating a skill's current level against a set of unlocked achievements.
///
/// Lv.1 is implicit: any positive `total_points` puts the skill at Lv.1, without a
/// declared threshold. `skill.level_thresholds` defines gates from Lv.2 onward.
#[derive(Debug, Clone)]
pub struct SkillLevelResult {
    pub current_level: u32,
    pub total_points: u32,
    pub max_points: u32,
}

/// Evaluate a skill's current level against the set of unlocked achievement IDs.
///
/// Legacy JSON skill-level computation retained for the old Agent model. The desktop
/// Skills screen now derives levels through `application::SkillCommands`.
pub fn compute_skill_level(skill: &SkillDef, unlocked_ids: &HashSet<String>) -> SkillLevelResult {
    let total_points: u32 = skill
        .nodes
        .iter()
        .filter(|n| unlocked_ids.contains(&n.achievement_id))
        .map(|n| n.points)
        .sum();
    let max_points: u32 = skill.nodes.iter().map(|n| n.points).sum();

    // Lv.1 is implicit: any positive points count as Lv.1. Thresholds define Lv.2+.
    let mut current_level: u32 = if total_points >= 1 { 1 } else { 0 };
    let mut accumulated_keys: Vec<&str> = Vec::new();
    for threshold in &skill.level_thresholds {
        accumulated_keys.extend(
            threshold
                .required_key_achievements
                .iter()
                .map(|s| s.as_str()),
        );
        let all_keys_unlocked = accumulated_keys.iter().all(|id| unlocked_ids.contains(*id));

        if total_points >= threshold.points_required && all_keys_unlocked {
            current_level = threshold.level;
        } else {
            break;
        }
    }

    SkillLevelResult {
        current_level,
        total_points,
        max_points,
    }
}

#[derive(Debug, Serialize)]
pub struct SkillWithLevel {
    pub skill: SkillDef,
    pub pack_id: String,
    pub pack_name: String,
    pub current_level: u32,
    pub current_points: u32,
    pub max_points: u32,
    pub next_threshold: Option<LevelThreshold>,
}

#[derive(Debug, Serialize)]
pub struct SkillData {
    pub skills: Vec<SkillWithLevel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_level_titles_5() {
        let titles = default_level_titles(5);
        println!("max_level=5: {:?}", titles);
        assert_eq!(titles.len(), 5);
    }

    #[test]
    fn test_default_level_titles_0() {
        let titles = default_level_titles(0);
        assert!(titles.is_empty());
    }

    #[test]
    fn test_default_level_titles_1() {
        let titles = default_level_titles(1);
        println!("max_level=1: {:?}", titles);
        assert_eq!(titles.len(), 1);
        assert_eq!(titles[0], "登峰造极");
    }

    fn make_skill(
        nodes: Vec<(&str, &str, u32)>,
        thresholds: Vec<(u32, u32, Vec<&str>)>,
    ) -> SkillDef {
        SkillDef {
            id: "test::skill".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            max_level: 5,
            level_titles: vec![],
            level_thresholds: thresholds
                .into_iter()
                .map(|(level, points, keys)| LevelThreshold {
                    level,
                    points_required: points,
                    required_key_achievements: keys.into_iter().map(String::from).collect(),
                })
                .collect(),
            nodes: nodes
                .into_iter()
                .map(|(node_id, ach, points)| SkillNode {
                    node_id: node_id.to_string(),
                    achievement_id: ach.to_string(),
                    points,
                })
                .collect(),
            card_image: None,
        }
    }

    #[test]
    fn level_is_zero_when_no_achievements_unlocked() {
        let skill = make_skill(
            vec![("n1", "a1", 5), ("n2", "a2", 10)],
            vec![(2, 20, vec![]), (3, 40, vec![])],
        );
        let result = compute_skill_level(&skill, &HashSet::new());
        assert_eq!(result.current_level, 0);
        assert_eq!(result.total_points, 0);
        assert_eq!(result.max_points, 15);
    }

    #[test]
    fn level_is_one_when_any_point_earned() {
        // Lv.1 is implicit: a single point is enough, no threshold needed.
        let skill = make_skill(
            vec![("n1", "a1", 1), ("n2", "a2", 50)],
            vec![(2, 20, vec![]), (3, 40, vec![])],
        );
        let unlocked: HashSet<String> = ["a1".to_string()].into_iter().collect();
        let result = compute_skill_level(&skill, &unlocked);
        assert_eq!(result.current_level, 1);
        assert_eq!(result.total_points, 1);
    }

    #[test]
    fn level_advances_through_thresholds() {
        let skill = make_skill(
            vec![("n1", "a1", 25), ("n2", "a2", 20)],
            vec![(2, 20, vec![]), (3, 40, vec![]), (4, 100, vec![])],
        );
        let unlocked: HashSet<String> = ["a1".to_string(), "a2".to_string()].into_iter().collect();
        let result = compute_skill_level(&skill, &unlocked);
        assert_eq!(result.total_points, 45);
        assert_eq!(result.current_level, 3); // 45 >= 40 but < 100
    }

    #[test]
    fn key_achievement_gate_blocks_level_up() {
        // Has enough points for Lv.2, but the required key achievement is missing.
        let skill = make_skill(
            vec![("n1", "a1", 30)],
            vec![(2, 20, vec!["gate_key"]), (3, 40, vec![])],
        );
        let unlocked: HashSet<String> = ["a1".to_string()].into_iter().collect();
        let result = compute_skill_level(&skill, &unlocked);
        assert_eq!(result.current_level, 1); // stuck at Lv.1 despite 30 points
    }

    #[test]
    fn key_achievements_are_incremental_accumulated() {
        // Lv.3's gate inherits Lv.2's gate. Missing Lv.2 key means Lv.3 blocked
        // even if we meet Lv.3's point requirement and its own key.
        let skill = make_skill(
            vec![("n1", "a1", 50)],
            vec![(2, 20, vec!["key_lv2"]), (3, 40, vec!["key_lv3"])],
        );
        let unlocked: HashSet<String> = ["a1".to_string(), "key_lv3".to_string()]
            .into_iter()
            .collect();
        let result = compute_skill_level(&skill, &unlocked);
        assert_eq!(result.current_level, 1); // can't reach Lv.2 → can't reach Lv.3
    }

    #[test]
    fn test_serialization_includes_level_titles() {
        let skill = SkillDef {
            id: "test::skill".to_string(),
            name: "Test Skill".to_string(),
            description: String::new(),
            max_level: 5,
            level_titles: default_level_titles(5),
            level_thresholds: vec![],
            nodes: vec![],
            card_image: None,
        };
        let json = serde_json::to_string(&skill).unwrap();
        println!("Serialized: {}", json);
        assert!(json.contains("level_titles"));
        assert!(json.contains("初窥门径"));
    }
}
