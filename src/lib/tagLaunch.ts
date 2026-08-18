import { Tag, TagCategory } from '@/types';

/**
 * Categories whose tags are expected to spawn a process. Tags outside this set
 * are labels only, so the launch surfaces must not offer them as runnable.
 */
export const LAUNCHABLE_CATEGORIES: TagCategory[] = ['ide', 'cli', 'startup'];

export function isLaunchableCategory(category: TagCategory): boolean {
    return LAUNCHABLE_CATEGORIES.includes(category);
}

/**
 * Split a command line the way a POSIX-ish shell would: honour single and
 * double quotes so pasted commands such as `claude --settings "/a b/c.json"`
 * keep the path in one token and drop the quote characters themselves.
 */
export function tokenizeCommandLine(input: string): string[] {
    const tokens: string[] = [];
    let current = '';
    let quote: '"' | "'" | null = null;
    let hasToken = false;

    for (let index = 0; index < input.length; index += 1) {
        const char = input[index];

        if (quote) {
            if (char === quote) {
                quote = null;
            } else if (quote === '"' && char === '\\' && index + 1 < input.length) {
                index += 1;
                current += input[index];
            } else {
                current += char;
            }
            continue;
        }

        if (char === '"' || char === "'") {
            quote = char;
            hasToken = true;
            continue;
        }

        if (char === '\\' && index + 1 < input.length) {
            index += 1;
            current += input[index];
            hasToken = true;
            continue;
        }

        if (/\s/.test(char)) {
            if (hasToken) {
                tokens.push(current);
                current = '';
                hasToken = false;
            }
            continue;
        }

        current += char;
        hasToken = true;
    }

    if (quote) {
        // Unbalanced quote: keep what the user typed rather than silently
        // dropping the rest of the line.
        tokens.push(current);
    } else if (hasToken) {
        tokens.push(current);
    }

    return tokens;
}

export interface NormalizedLaunchInput {
    executable: string;
    args: string[];
    /** True when the executable was taken from the arguments field. */
    promotedFromArgs: boolean;
}

/**
 * Accept either a filled-in executable plus arguments, or a whole command line
 * pasted into the arguments field, and return the split VibeHub stores.
 */
export function normalizeLaunchInput(
    executableInput: string,
    argsInput: string
): NormalizedLaunchInput {
    const executableTokens = tokenizeCommandLine(executableInput);
    const argTokens = tokenizeCommandLine(argsInput);

    if (executableTokens.length > 1) {
        return {
            executable: executableTokens[0],
            args: [...executableTokens.slice(1), ...argTokens],
            promotedFromArgs: false,
        };
    }

    if (executableTokens.length === 1) {
        return { executable: executableTokens[0], args: argTokens, promotedFromArgs: false };
    }

    if (argTokens.length > 0) {
        return {
            executable: argTokens[0],
            args: argTokens.slice(1),
            promotedFromArgs: true,
        };
    }

    return { executable: '', args: [], promotedFromArgs: false };
}

export type TagLaunchIssue = 'not_launchable_category' | 'missing_executable';

/**
 * The single source of truth for "can VibeHub actually start this tag?".
 * Every launch surface must use this so the context menu, the custom launch
 * dialog and the backend agree instead of failing late.
 */
export function tagLaunchIssue(tag: Tag): TagLaunchIssue | null {
    if (!isLaunchableCategory(tag.category)) return 'not_launchable_category';
    if (!tag.config?.executable?.trim()) return 'missing_executable';
    return null;
}

export function isTagLaunchable(tag: Tag): boolean {
    return tagLaunchIssue(tag) === null;
}
