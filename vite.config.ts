import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import { viteStaticCopy } from "vite-plugin-static-copy";
import path from "path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async ({ mode }) => {
    const env = loadEnv(mode, process.cwd(), "");
    const includeV3Fixtures = mode !== "production" || env.VITE_V3_DEBUG === "true";

    return {
        plugins: [
            react(),
            ...(includeV3Fixtures ? [viteStaticCopy({
                targets: [
                    {
                        src: "fixtures/v3/*",
                        dest: "fixtures/v3",
                    },
                ],
            })] : []),
        ],

        resolve: {
            alias: {
                "@": path.resolve(__dirname, "./src"),
            },
        },

        clearScreen: false,
        server: {
            port: 1420,
            strictPort: true,
            host: host || false,
            hmr: host
                ? {
                    protocol: "ws",
                    host,
                    port: 1421,
                }
                : undefined,
            watch: {
                ignored: ["**/src-tauri/**"],
            },
        },
    };
});
