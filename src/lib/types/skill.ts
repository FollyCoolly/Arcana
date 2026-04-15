export type SkillNode = {
  node_id: string;
  achievement_id: string;
  points: number;
};

export type LevelThreshold = {
  level: number;
  points_required: number;
  required_key_achievements: string[];
};

export type SkillDef = {
  id: string;
  name: string;
  description: string;
  max_level: number;
  level_titles?: string[];
  level_thresholds: LevelThreshold[];
  nodes: SkillNode[];
};

export type SkillWithLevel = {
  skill: SkillDef;
  pack_id: string;
  pack_name: string;
  current_level: number;
  current_points: number;
  max_points: number;
  next_threshold: LevelThreshold | null;
};

export type SkillData = {
  skills: SkillWithLevel[];
};
