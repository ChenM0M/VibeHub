import React from 'react';
import { Sidebar } from './Sidebar';
import { Header } from './Header';

type PageType = 'home' | 'settings' | 'gateway' | 'about';

interface LayoutProps {
    children: React.ReactNode;
    onSearch: (query: string) => void;
    currentPage: PageType;
    onNavigate: (page: PageType) => void;
    onCheckUpdate?: () => void;
    isCheckingUpdate?: boolean;
}

export function Layout({ children, onSearch, currentPage, onNavigate, onCheckUpdate, isCheckingUpdate }: LayoutProps) {
    return (
        <div className="flex h-screen w-full bg-background text-foreground overflow-hidden">
            <Sidebar
                currentPage={currentPage}
                onNavigate={onNavigate}
                onCheckUpdate={onCheckUpdate}
                isCheckingUpdate={isCheckingUpdate}
            />
            <div className="flex-1 flex flex-col min-w-0">
                <Header onSearch={onSearch} />
                <main className="flex-1 overflow-y-auto p-6 scroll-smooth">
                    {children}
                </main>
            </div>
        </div>
    );
}

