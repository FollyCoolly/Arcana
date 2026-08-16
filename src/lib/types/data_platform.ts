export type DataCommandError = {
    code: string;
    message: string;
};

export type PackAssetContent = {
    path: string;
    media_type: string;
    content: number[];
};

export function dataCommandErrorMessage(
    error: unknown,
    fallback: string,
): string {
    if (typeof error === "string") return error;
    if (
        error &&
        typeof error === "object" &&
        "message" in error &&
        typeof (error as DataCommandError).message === "string"
    ) {
        return (error as DataCommandError).message;
    }
    return fallback;
}
