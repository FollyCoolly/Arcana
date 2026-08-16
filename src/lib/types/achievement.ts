export type Difficulty = 'beginner' | 'intermediate' | 'advanced' | 'expert' | 'legendary';

export type Achievement = {
    id: string;
    name: string;
    description: string;
    difficulty: Difficulty;
    tags: string[];
    prerequisites: string[];
    related_record_definition_ids: string[];
    tip?: string;
    enabled: boolean;
    availability:
        | "locked"
        | "available"
        | "tracked"
        | "achieved"
        | "unresolved";
    unmet_prerequisite_ids: string[];
};

export type AchievementStatus = 'tracked' | 'achieved';

export type AchievementProgress = {
    status: AchievementStatus;
    achieved_at?: string;
};

export type PackAchievements = {
    pack_id: string;
    pack_name: string;
    achievements: Achievement[];
};

export type AchievementData = {
    packs: PackAchievements[];
    progress: Record<string, AchievementProgress>;
    unresolved_achievement_ids?: string[];
};
