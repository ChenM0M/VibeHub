/** Resolve the persisted snapshot before an action that reads native config. */
export async function profileForAction<T>(
    current: T | null,
    dirty: boolean,
    save: () => Promise<T | null>,
): Promise<T | null> {
    if (!current) return null;
    return dirty ? await save() : current;
}
