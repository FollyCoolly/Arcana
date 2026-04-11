export type StatusMetric = {
    id: string;
    name: string;
    group: string;
    unit: string;
    value_type: string;
    value: number | null;
    description?: string;
};

export type DimensionMetricResult = {
    metric_id: string;
    value: number | null;
    contribution: number | null;
    weight: number;
};

export type DimensionData = {
    id: string;
    name: string;
    level_titles: string[];
    level_thresholds: number[];
    enabled: boolean;
    score: number | null;
    level: number | null;
    level_title: string | null;
    metrics: DimensionMetricResult[];
};

export type StatusData = {
    definition_version: number;
    value_version: number;
    username: string;
    game_days: number | null;
    metrics: StatusMetric[];
    dimensions: DimensionData[];
    system_metrics: Record<string, number>;
};

export type MetricGroup = {
    name: string;
    metrics: StatusMetric[];
};
