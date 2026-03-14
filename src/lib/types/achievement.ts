export type Difficulty = 'beginner' | 'intermediate' | 'advanced' | 'expert' | 'legendary';

export type Achievement = {
    id: string;
    name: string;
    description: string;
    difficulty: Difficulty;
    category: string;
    tags: string[];
    prerequisites: string[];
};

export type UnlockInfo = {
    achieved_at?: string;
    note?: string;
};

export type PackAchievements = {
    pack_id: string;
    pack_name: string;
    achievements: Achievement[];
};

export type AchievementData = {
    packs: PackAchievements[];
    progress: Record<string, UnlockInfo>;
};
