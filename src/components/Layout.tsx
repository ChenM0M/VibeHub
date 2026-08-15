import React from 'react';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { ProjectTabBar } from './ProjectTabBar';
import { Activity, Bot, LayoutGrid, Settings } from 'lucide-react';
import { useTranslation } from 'react-i18next';

type PageType = 'home' | 'settings' | 'gateway' | 'agent-profiles' | 'about';

interface LayoutProps {
    children: React.ReactNode;
    onSearch: (query: string) => void;
    currentPage: PageType;
    onNavigate: (page: PageType) => void;
    onCheckUpdate?: () => void;
    isCheckingUpdate?: boolean;
}

export function Layout({ children, onSearch, currentPage, onNavigate, onCheckUpdate, isCheckingUpdate }: LayoutProps) {
    const { t } = useTranslation();

    return (
        <div className="flex h-screen w-full bg-background text-foreground overflow-hidden">
            <Sidebar
                className="hidden shrink-0 md:flex"
                currentPage={currentPage}
                onNavigate={onNavigate}
                onCheckUpdate={onCheckUpdate}
                isCheckingUpdate={isCheckingUpdate}
            />
            <div className="flex-1 flex flex-col min-w-0">
                <nav className="flex h-12 shrink-0 items-center justify-between border-b border-border/40 bg-background/90 px-3 backdrop-blur md:hidden" aria-label="移动端主导航">
                    <button type="button" className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onClick={() => onNavigate('home')}>
                        <span className="grid h-6 w-6 place-items-center rounded-md bg-primary text-[10px] text-primary-foreground">V</span>
                        VibeHub
                    </button>
                    <div className="flex items-center gap-1">
                        <button type="button" aria-label="工作区" className="rounded-md p-2 text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onClick={() => onNavigate('home')}><LayoutGrid className="h-4 w-4" /></button>
                        <button type="button" aria-label="AI 网关" className="rounded-md p-2 text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onClick={() => onNavigate('gateway')}><Activity className="h-4 w-4" /></button>
                        <button type="button" aria-label={t('agentProfiles.navLabel')} className={currentPage === 'agent-profiles' ? "rounded-md bg-accent p-2 text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" : "rounded-md p-2 text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"} onClick={() => onNavigate('agent-profiles')}><Bot className="h-4 w-4" /></button>
                        <button type="button" aria-label="设置" className="rounded-md p-2 text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onClick={() => onNavigate('settings')}><Settings className="h-4 w-4" /></button>
                    </div>
                </nav>
                <Header onSearch={onSearch} />
                {currentPage === 'home' && <ProjectTabBar />}
                <main className="flex-1 overflow-y-auto p-4 scroll-smooth scrollbar-auto-hide overscroll-none md:p-6">
                    {children}
                </main>
            </div>
        </div>
    );
}
