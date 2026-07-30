import { create } from 'zustand';

interface TabsState {
    tabs: string[];
    activeTabId: string | null;
    openTab: (projectId: string) => void;
    activateTab: (projectId: string) => void;
    activateIndex: (index: number) => void;
    activateRelative: (offset: number) => void;
    closeTab: (projectId: string) => void;
    closeActiveTab: () => void;
    reorderTabs: (tabs: string[]) => void;
    deactivateTabs: () => void;
    pruneTabs: (validProjectIds: string[]) => void;
}

export const useTabsStore = create<TabsState>((set, get) => ({
    tabs: [],
    activeTabId: null,

    openTab: (projectId) => {
        const { tabs } = get();
        set({
            tabs: tabs.includes(projectId) ? tabs : [...tabs, projectId],
            activeTabId: projectId,
        });
    },

    activateTab: (projectId) => {
        if (!get().tabs.includes(projectId)) return;
        set({ activeTabId: projectId });
    },

    activateIndex: (index) => {
        const { tabs } = get();
        if (index < 0 || index >= tabs.length) return;
        set({ activeTabId: tabs[index] });
    },

    activateRelative: (offset) => {
        const { tabs, activeTabId } = get();
        if (tabs.length === 0) return;
        const current = activeTabId ? tabs.indexOf(activeTabId) : -1;
        const base = current === -1 ? 0 : current;
        const next = (base + offset + tabs.length) % tabs.length;
        set({ activeTabId: tabs[next] });
    },

    closeTab: (projectId) => {
        const { tabs, activeTabId } = get();
        const index = tabs.indexOf(projectId);
        if (index === -1) return;
        const remaining = tabs.filter((id) => id !== projectId);
        if (activeTabId !== projectId) {
            set({ tabs: remaining });
            return;
        }
        const neighbour = remaining[index] ?? remaining[index - 1] ?? null;
        set({ tabs: remaining, activeTabId: neighbour });
    },

    closeActiveTab: () => {
        const { activeTabId, closeTab } = get();
        if (!activeTabId) return;
        closeTab(activeTabId);
    },

    reorderTabs: (tabs) => set({ tabs }),

    deactivateTabs: () => set({ activeTabId: null }),

    pruneTabs: (validProjectIds) => {
        const { tabs, activeTabId } = get();
        const valid = new Set(validProjectIds);
        const remaining = tabs.filter((id) => valid.has(id));
        if (remaining.length === tabs.length) return;
        set({
            tabs: remaining,
            activeTabId: activeTabId && valid.has(activeTabId) ? activeTabId : null,
        });
    },
}));
