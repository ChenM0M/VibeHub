import { Search, Moon, Sun, Bell } from 'lucide-react';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { useAppStore } from '@/stores/appStore';

interface HeaderProps {
    onSearch: (query: string) => void;
}

export function Header({ onSearch }: HeaderProps) {
    const { config, setTheme } = useAppStore();
    const isDark = config?.theme === 'dark';

    const toggleTheme = () => {
        setTheme(isDark ? 'light' : 'dark');
    };

    return (
        <header className="h-14 border-b border-border/50 flex items-center px-6 glass sticky top-0 z-10">
            <div className="flex-1 max-w-xl relative">
                <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
                <Input
                    placeholder="搜索项目..."
                    className="pl-10 bg-secondary/30 backdrop-blur-sm border-border/50 focus-visible:bg-background/80 focus-visible:border-primary/50 transition-all rounded-lg shadow-sm"
                    onChange={(e) => onSearch(e.target.value)}
                />
            </div>

            <div className="ml-auto flex items-center gap-2">
                <Button
                    variant="ghost"
                    size="icon"
                    onClick={toggleTheme}
                    className="rounded-lg hover:bg-primary/10 transition-colors"
                >
                    {isDark ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
                </Button>
                <Button
                    variant="ghost"
                    size="icon"
                    className="rounded-lg hover:bg-primary/10 transition-colors"
                >
                    <Bell className="h-4 w-4" />
                </Button>
            </div>
        </header>
    );
}
