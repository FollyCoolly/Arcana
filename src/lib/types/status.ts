export type StatusMetric = {
    id: string;
    name: string;
    category: "health" | "performance" | string;
    group: string;
    sub_group?: string;
    unit: string;
    value_type: string;
    value: number | null;
    target_max?: number;
    target_min?: number;
    body_parts: string[];
    description?: string;
};

export type StatusData = {
    definition_version: number;
    value_version: number;
    username: string;
    game_days: number | null;
    bmi: number | null;
    metrics: StatusMetric[];
};

export type MetricGroup = {
    name: string;
    metrics: StatusMetric[];
};
