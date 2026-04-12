import { useState, useRef, useEffect } from 'react';
import { Search, Moon, Sun, Bell, Minus, Square, X } from 'lucide-react';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { useAppStore } from '@/stores/appStore';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface HeaderProps {
    onSearch: (query: string) => void;
}

export function Header({ onSearch }: HeaderProps) {
    const { config, setTheme } = useAppStore();
    const isDark = config?.theme === 'dark';
    const [searchValue, setSearchValue] = useState('');
    const inputRef = useRef<HTMLInputElement>(null);
    const lastMouseTargetRef = useRef<EventTarget | null>(null);

    useEffect(() => {
        const handler = (e: MouseEvent) => {
            lastMouseTargetRef.current = e.target;
        };
        document.addEventListener('mousedown', handler);
        return () => document.removeEventListener('mousedown', handler);
    }, []);

    const toggleTheme = () => {
        setTheme(isDark ? 'light' : 'dark');
    };

    const handleMinimize = async () => {
        try {
            await getCurrentWindow().minimize();
        } catch (e) {
            console.error('Minimize failed:', e);
        }
    };

    const handleMaximize = async () => {
        try {
            const win = getCurrentWindow();
            const isMaximized = await win.isMaximized();
            if (isMaximized) {
                await win.unmaximize();
            } else {
                await win.maximize();
            }
        } catch (e) {
            console.error('Maximize failed:', e);
        }
    };

    const handleClose = async () => {
        try {
            await getCurrentWindow().close();
        } catch (e) {
            console.error('Close failed:', e);
        }
    };

    const handleDragStart = async (e: React.MouseEvent) => {
        // Only start drag on left mouse button and not on interactive elements
        if (e.button !== 0) return;
        const target = e.target as HTMLElement;
        if (target.closest('button, input, a')) return;

        try {
            await getCurrentWindow().startDragging();
        } catch (err) {
            console.error('Drag failed:', err);
        }
    };

    return (
        <header
            className="h-12 border-b border-border/50 flex items-center glass sticky top-0 z-10 select-none"
            onMouseDown={handleDragStart}
        >
            <div className="flex-1 max-w-xl relative ml-4">
                <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground pointer-events-none" />
                <Input
                    ref={inputRef}
                    value={searchValue}
                    placeholder="搜索项目..."
                    className="pl-10 pr-8 h-9 bg-secondary/30 backdrop-blur-sm border-border/50 focus-visible:bg-background/80 focus-visible:border-primary/50 transition-all rounded-lg shadow-sm"
                    onChange={(e) => { setSearchValue(e.target.value); onSearch(e.target.value); }}
                    onBlur={() => {
                        const target = lastMouseTargetRef.current as HTMLElement | null;
                        if (target?.closest('[data-project-card], button, input, a, [role="menuitem"]')) {
                            return;
                        }
                        setSearchValue('');
                        onSearch('');
                    }}
                    onMouseDown={(e) => e.stopPropagation()}
                />
                {searchValue && (
                    <button
                        className="absolute right-2 top-2.5 h-4 w-4 text-muted-foreground hover:text-foreground transition-colors"
                        onClick={() => { setSearchValue(''); onSearch(''); inputRef.current?.focus(); }}
                        onMouseDown={(e) => e.stopPropagation()}
                    >
                        <X className="h-4 w-4" />
                    </button>
                )}
            </div>

            <div className="ml-auto flex items-center gap-1 pr-1">
                <Button
                    variant="ghost"
                    size="icon"
                    onClick={toggleTheme}
                    onMouseDown={(e) => e.stopPropagation()}
                    className="h-8 w-8 rounded-lg hover:bg-primary/10 transition-colors"
                >
                    {isDark ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
                </Button>
                <Button
                    variant="ghost"
                    size="icon"
                    onMouseDown={(e) => e.stopPropagation()}
                    className="h-8 w-8 rounded-lg hover:bg-primary/10 transition-colors"
                >
                    <Bell className="h-4 w-4" />
                </Button>

                {/* Window controls */}
                <div className="flex items-center ml-2 border-l border-border/50 pl-2">
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={handleMinimize}
                        onMouseDown={(e) => e.stopPropagation()}
                        className="h-8 w-8 rounded-md hover:bg-muted transition-colors"
                    >
                        <Minus className="h-4 w-4" />
                    </Button>
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={handleMaximize}
                        onMouseDown={(e) => e.stopPropagation()}
                        className="h-8 w-8 rounded-md hover:bg-muted transition-colors"
                    >
                        <Square className="h-3.5 w-3.5" />
                    </Button>
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={handleClose}
                        onMouseDown={(e) => e.stopPropagation()}
                        className="h-8 w-8 rounded-md hover:bg-destructive hover:text-destructive-foreground transition-colors"
                    >
                        <X className="h-4 w-4" />
                    </Button>
                </div>
            </div>
        </header>
    );
}
