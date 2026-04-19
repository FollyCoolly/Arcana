export type ItemSourceInfo = {
    id: string;
    name: string;
    item_count: number;
};

export type ItemWithComputed = {
    id: string;
    source_id: string;
    name: string;
    brand: string | null;
    price: number | null;
    purchase_date: string | null;
    purchase_channel: string | null;
    category: string | null;
    color: string | null;
    image: string | null;
    extra: Record<string, unknown>;
    days_owned: number | null;
    daily_cost: number | null;
};

export type SourceStats = {
    source_id: string;
    source_name: string;
    item_count: number;
    total_value: number;
};

export type ItemStats = {
    total_items: number;
    total_value: number;
    average_daily_cost: number;
    by_source: SourceStats[];
};

export type ItemData = {
    sources: ItemSourceInfo[];
    items: ItemWithComputed[];
    stats: ItemStats;
};

export type ItemSortKey = 'name' | 'price' | 'daily_cost' | 'date' | 'days_owned';
export type ItemSortOrder = 'asc' | 'desc';
