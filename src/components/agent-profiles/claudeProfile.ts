export const CLAUDE_AUTO_MODEL_VALUE = '__auto__' as const;

export type ClaudeModelSelectionStatus = 'auto' | 'available' | 'unknown';

export type ClaudeModelSelection = {
    uiValue: string;
    payloadValue: string | null;
    status: ClaudeModelSelectionStatus;
    options: string[];
};

function cleanModelValues(values: readonly string[]): string[] {
    return Array.from(new Set(values.map((value) => value.trim()).filter((value) => value && value !== CLAUDE_AUTO_MODEL_VALUE)));
}

/**
 * Canonicalizes the Claude advanced model value at the React boundary.
 * `__auto__` is a controlled-select sentinel only; the saved profile payload
 * uses null so the adapter can apply the managed default-model fallback.
 * Unknown existing values remain visible and preserved instead of becoming a
 * blank controlled select.
 */
export function normalizeClaudeModelSelection(value: unknown, availableModels: readonly string[]): ClaudeModelSelection {
    const normalized = typeof value === 'string' ? value.trim() : '';
    const options = cleanModelValues(availableModels);
    if (!normalized || normalized === CLAUDE_AUTO_MODEL_VALUE) {
        return {
            uiValue: CLAUDE_AUTO_MODEL_VALUE,
            payloadValue: null,
            status: 'auto',
            options: [CLAUDE_AUTO_MODEL_VALUE, ...options],
        };
    }
    const status: ClaudeModelSelectionStatus = options.includes(normalized) ? 'available' : 'unknown';
    return {
        uiValue: normalized,
        payloadValue: normalized,
        status,
        options: status === 'unknown' ? [CLAUDE_AUTO_MODEL_VALUE, ...options, normalized] : [CLAUDE_AUTO_MODEL_VALUE, ...options],
    };
}
