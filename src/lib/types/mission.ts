export type MissionResponse = {
    id: string;
    title: string;
    description?: string;
    status: string;
    progress?: number;
    deadline?: string;
    linked_achievement_id?: string;
    created_at?: string;
    completed_at?: string;
    days_remaining?: number;
};

export type MissionData = {
    missions: MissionResponse[];
};

export type CountdownDisplay = {
    label: string;
    days_remaining: number;
};

export type ProgressDisplay = {
    label: string;
    progress: number;
};

export type MainMenuMissionData = {
    countdown: CountdownDisplay | null;
    progress: ProgressDisplay | null;
};
