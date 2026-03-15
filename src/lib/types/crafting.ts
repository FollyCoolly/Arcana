export type RecipeSourceInfo = {
    id: string;
    name: string;
    icon: string;
    recipe_count: number;
};

export type RecipeWithComputed = {
    id: string;
    source_id: string;
    name: string;
    recipe_type: string | null;
    servings: string | null;
    difficulty: string | null;
    time: string | null;
    ingredients: string[];
    steps: string[];
    tags: string[];
    source: string | null;
    image: string | null;
    extra: Record<string, unknown>;
    ingredient_count: number;
    step_count: number;
};

export type RecipeSourceStats = {
    source_id: string;
    source_name: string;
    source_icon: string;
    recipe_count: number;
};

export type RecipeTypeStats = {
    name: string;
    recipe_count: number;
};

export type RecipeStats = {
    total_recipes: number;
    by_source: RecipeSourceStats[];
    by_type: RecipeTypeStats[];
};

export type CraftingData = {
    sources: RecipeSourceInfo[];
    recipes: RecipeWithComputed[];
    stats: RecipeStats;
};
