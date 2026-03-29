export type Difficulty = 'beginner' | 'intermediate' | 'advanced' | 'expert' | 'legendary';

export type Achievement = {
    id: string;
    name: string;
    description: string;
    difficulty: Difficulty;
    tags: string[];
    prerequisites: string[];
};

export type AchievementStatus = 'tracked' | 'achieved';

export type AchievementProgress = {
    status: AchievementStatus;
    achieved_at?: string;
    tracked_at?: string;
    note?: string;
    progress_detail?: string[];
    may_be_incomplete?: boolean;
};

export type PackAchievements = {
    pack_id: string;
    pack_name: string;
    achievements: Achievement[];
};

export type AchievementData = {
    packs: PackAchievements[];
    progress: Record<string, AchievementProgress>;
};
