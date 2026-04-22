export interface UiEvent {
    id: string;
    type: string;
    timestamp: string;
    data: Record<string, unknown>;
}
