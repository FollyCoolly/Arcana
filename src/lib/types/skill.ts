import type { Achievement } from "$lib/types/achievement";

export type AchievementAvailability = Achievement["availability"];

export type SkillNode = {
    achievement_id: string;
    points: number;
};

export type SkillNodeEvaluation = SkillNode & {
    availability: AchievementAvailability;
};

export type SkillDef = {
    id: string;
    name: string;
    description?: string;
    level_thresholds: number[];
    nodes: SkillNode[];
    card_image?: string;
};

export type SkillWithLevel = {
    pack_id: string;
    pack_name: string;
    definition: SkillDef;
    points: number;
    max_points: number;
    level: number;
    achieved_node_count: number;
    node_count: number;
    nodes: SkillNodeEvaluation[];
};

export type SkillData = {
    skills: SkillWithLevel[];
};
