type ViteImportMeta = ImportMeta & {
    env: {
        DEV?: boolean;
        VITE_START_PAGE?: string;
        VITE_V3_DEBUG?: string;
    };
};

export const viteEnv = (import.meta as ViteImportMeta).env;
