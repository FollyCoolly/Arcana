export type PackSummary = {
    id: string;
    name: string;
    enabled: boolean;
    parent_pack_id?: string;
    tags: string[];
    record_definition_count: number;
    dimension_count: number;
    achievement_count: number;
    skill_count: number;
    asset_count: number;
};

export type PackDashboardData = {
    packs: PackSummary[];
};

export type PackEnabledState = {
    pack_id: string;
    enabled: boolean;
    changed: boolean;
};

export type PackDeleteResult = {
    pack_id: string;
    was_enabled: boolean;
    child_pack_ids: string[];
    unresolved_record_ids: string[];
    unresolved_achievement_state_ids: string[];
    orphaned_status_dimension_ids: string[];
};
