export function parseVariantObject(text: string): Record<string, unknown> {
    const value: unknown = JSON.parse(text);
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('variantObjectRequired');
    const object = value as Record<string, unknown>;
    return object;
}
export function parseVariantDrafts(names: string[], drafts: Record<string, string>, previous: Record<string, unknown> | null): Record<string, unknown> {
    return Object.fromEntries(names.map(name => [name, validateVariantObject(drafts[name] ?? JSON.stringify(previous?.[name] ?? {}))]));
}

function validateVariantObject(text: string): Record<string, unknown> {
    const object = parseVariantObject(text);
    const thinking = object.thinking;
    if (thinking && typeof thinking === 'object' && !Array.isArray(thinking)) {
        const budget = (thinking as Record<string, unknown>).budgetTokens;
        if (((thinking as Record<string, unknown>).type === 'enabled' || budget !== undefined) && (!Number.isSafeInteger(budget) || Number(budget) < 1024)) throw new Error('invalidBudget');
    }
    return object;
}
