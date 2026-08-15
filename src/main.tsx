import React, { useEffect, useState } from 'react';
import ReactDOM from 'react-dom/client';
import { Layout } from '@/components/Layout';
import { Home } from '@/pages/Home';
import { Settings } from '@/pages/Settings';
import { AgentProfiles } from '@/pages/AgentProfiles';
import { Gateway } from '@/pages/Gateway';
import { About } from '@/pages/About';
import { UpdateChecker } from '@/components/UpdateChecker';
import { useAppStore } from '@/stores/appStore';
import '@/styles/globals.css';
import './i18n';

export type PageType = 'home' | 'settings' | 'gateway' | 'agent-profiles' | 'about';

function pageFromHash(): PageType {
    const value = window.location.hash.replace(/^#/, '');
    return value === 'settings' || value === 'gateway' || value === 'agent-profiles' || value === 'about'
        ? value
        : 'home';
}

function App() {
    const { initializeApp } = useAppStore();
    const [currentPage, setCurrentPage] = useState<PageType>(() => pageFromHash());
    const [homeResetKey, setHomeResetKey] = useState(0);
    const [searchQuery, setSearchQuery] = useState('');
    const [triggerUpdateCheck, setTriggerUpdateCheck] = useState(false);
    const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);

    useEffect(() => {
        initializeApp();
        const handleHashChange = () => setCurrentPage(pageFromHash());
        window.addEventListener('hashchange', handleHashChange);
        return () => window.removeEventListener('hashchange', handleHashChange);
    }, []);

    const handleCheckUpdate = () => {
        setIsCheckingUpdate(true);
        setTriggerUpdateCheck(true);
    };

    const handleUpdateCheckComplete = () => {
        setTriggerUpdateCheck(false);
        setIsCheckingUpdate(false);
    };

    const handleNavigate = (page: PageType) => {
        if (page === 'home') {
            setHomeResetKey((key) => key + 1);
        }
        setCurrentPage(page);
        const nextHash = page === 'home' ? '' : `#${page}`;
        if (window.location.hash !== nextHash) {
            window.history.replaceState(null, '', `${window.location.pathname}${window.location.search}${nextHash}`);
        }
    };

    return (
        <>
            <UpdateChecker
                showManualCheckResult={triggerUpdateCheck}
                onManualCheckComplete={handleUpdateCheckComplete}
            />
            <Layout
                onSearch={setSearchQuery}
                currentPage={currentPage}
                onNavigate={handleNavigate}
                onCheckUpdate={handleCheckUpdate}
                isCheckingUpdate={isCheckingUpdate}
            >
                {currentPage === 'home' && <Home searchQuery={searchQuery} resetKey={homeResetKey} />}
                {currentPage === 'settings' && <Settings />}
                {currentPage === 'gateway' && <Gateway />}
                {currentPage === 'agent-profiles' && <AgentProfiles />}
                {currentPage === 'about' && <About />}
            </Layout>
        </>
    );
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
        <App />
    </React.StrictMode>
);
