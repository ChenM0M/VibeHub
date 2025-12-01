import React from 'react';
import { Sidebar } from './Sidebar';
import { Header } from './Header';

interface LayoutProps {
    children: React.ReactNode;
    onSearch: (query: string) => void;
    currentPage: 'home' | 'settings' | 'gateway';
    onNavigate: (page: 'home' | 'settings' | 'gateway') => void;
}

export function Layout({ children, onSearch, currentPage, onNavigate }: LayoutProps) {
    return (
        <div className="flex h-screen w-full bg-background text-foreground overflow-hidden">
            <Sidebar
                currentPage={currentPage}
                onNavigate={onNavigate}
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
