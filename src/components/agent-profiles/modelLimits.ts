import type { ModelLimits } from '@/v3/contracts/generated/agent-profile';
export type ModelLimitDraft = Record<'context' | 'input' | 'output', string>;
export function parseModelLimits(draft: ModelLimitDraft): ModelLimits | null {
    const parse = (value: string) => value.trim() === '' ? null : Number(value);
    const values: ModelLimits = { context: parse(draft.context), input: parse(draft.input), output: parse(draft.output) };
    if (Object.values(values).every(v => v === null)) return null;
    if (values.context === null || values.output === null || Object.values(values).some(v => v !== null && (!Number.isSafeInteger(v) || v < 1))) throw new Error('invalidLimits');
    return values;
}
