export type StatusScoreData = {
    id: string;
    name: string;
    weight: number;
    expression: string;
    raw_value: number | null;
    score: number | null;
    missing_record_ids?: string[];
};

export type DimensionData = {
    pack_id: string;
    id: string;
    name: string;
    level_titles: string[];
    level_thresholds: number[];
    selected_position?: number;
    score: number | null;
    level: number;
    level_title?: string;
    scores: StatusScoreData[];
};

export type StatusData = {
    username: string;
    game_days: number | null;
    dimensions: DimensionData[];
};
