import { useEffect, useRef } from 'react';
import { viteEnv } from '@/lib/viteEnv';

const CUE_URL = 'http://127.0.0.1:18765/screenshot-command';

export function useDevScreenshotCue(handler: (command: string) => void) {
    const handlerRef = useRef(handler);
    handlerRef.current = handler;

    useEffect(() => {
        if (!viteEnv.DEV) {
            return undefined;
        }
        let cancelled = false;
        const poll = async () => {
            try {
                const response = await fetch(CUE_URL, { cache: 'no-store' });
                if (!response.ok) return;
                const body = (await response.json()) as { command?: string | null };
                if (!cancelled && body.command) {
                    handlerRef.current(body.command);
                }
            } catch {
                // Screenshot control server is optional.
            }
        };
        const timer = window.setInterval(poll, 400);
        void poll();
        return () => {
            cancelled = true;
            window.clearInterval(timer);
        };
    }, []);
}
