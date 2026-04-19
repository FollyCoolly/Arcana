use serde::{Deserialize, Serialize};

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
