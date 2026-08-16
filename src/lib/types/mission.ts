export type MissionStatus = "active" | "completed" | "archived";
export type MissionDifficulty = "S" | "A" | "B" | "C" | "D";
export type MissionSuggestionStatus = "pending" | "rejected";
export type DashboardMissionSlot =
    | "countdown"
    | "progress"
    | "hint_1"
    | "hint_2";

export type Mission = {
    id: string;
    title: string;
    description?: string;
    status: MissionStatus;
    progress?: number;
    difficulty?: MissionDifficulty;
    deadline?: string;
    parent_id?: string;
    created_at: string;
    completed_at?: string;
    days_remaining?: number;
};

export type MissionSuggestion = {
    id: string;
    title: string;
    description?: string;
    difficulty?: MissionDifficulty;
    deadline?: string;
    parent_mission_id?: string;
    reason?: string;
    generated_at: string;
    status: MissionSuggestionStatus;
    days_remaining?: number;
};

export type MissionDashboardData = {
    missions: Mission[];
    suggestions: MissionSuggestion[];
};

export type CountdownDisplay = {
    label: string;
    short_desc: string;
    days_remaining: number;
};

export type HintDisplay = {
    short_desc: string;
};

export type ProgressDisplay = {
    label: string;
    progress: number;
};

export type DashboardMissionSelection = {
    mission_id: string;
    label?: string;
};

export type MissionMenuDashboardData = {
    countdown: CountdownDisplay | null;
    hints: HintDisplay[];
    progress: ProgressDisplay | null;
    selections: Partial<
        Record<DashboardMissionSlot, DashboardMissionSelection>
    >;
    unresolved_slots: DashboardMissionSlot[];
};
