export function formatGroupName(group: string): string {
    return group
        .split("_")
        .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
        .join(" ");
}

export function formatValue(value: number): string {
    return Number.isInteger(value) ? String(value) : value.toFixed(1).replace(/\.0$/, "");
}

export function formatUnit(unit: string): string {
    switch (unit) {
        case "":
        case "reps":
            return "";
        case "percent":
            return "%";
        case "kg_each":
            return "ea";
        case "sec":
            return "s";
        case "sec_per_km":
            return "/km";
        default:
            return unit.replace(/_/g, " ");
    }
}

export function secsToMSS(totalSecs: number): string {
    const m = Math.floor(totalSecs / 60);
    const s = Math.round(totalSecs % 60);
    return `${m}:${String(s).padStart(2, "0")}`;
}

export function formatMetricValue(value: number | null, unit: string): string {
    if (value === null || value === undefined) {
        return "--";
    }

    if (unit === "sec_per_km") {
        return `${secsToMSS(value)} /km`;
    }

    if (unit === "sec" && value >= 60) {
        return secsToMSS(value);
    }

    const unitStr = formatUnit(unit);
    const numStr = formatValue(value);

    if (!unitStr) {
        return numStr;
    }
    if (unitStr === "%") {
        return `${numStr}%`;
    }
    return `${numStr} ${unitStr}`;
}
