export type GallerySourceInfo = {
    id: string;
    name: string;
    icon: string;
    media_type: string;
    item_count: number;
};

export type MediaItem = {
    id: string;
    source_id: string;
    name: string;
    name_original: string | null;
    cover: string | null;
    rating: number | null;
    my_rating: number | null;
    date_started: string | null;
    date_finished: string | null;
    tags: string[];
    episodes: number | null;
    extra: Record<string, unknown>;
};

export type GallerySourceStats = {
    source_id: string;
    source_name: string;
    source_icon: string;
    item_count: number;
};

export type GalleryStats = {
    total_items: number;
    by_source: GallerySourceStats[];
};

export type GalleryData = {
    sources: GallerySourceInfo[];
    items: MediaItem[];
    stats: GalleryStats;
};
