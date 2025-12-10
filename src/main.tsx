import React, { useEffect, useState } from 'react';
import ReactDOM from 'react-dom/client';
import { Layout } from '@/components/Layout';
import { Home } from '@/pages/Home';
import { Settings } from '@/pages/Settings';
import { Gateway } from '@/pages/Gateway';
import { About } from '@/pages/About';
import { UpdateChecker } from '@/components/UpdateChecker';
import { useAppStore } from '@/stores/appStore';
import '@/styles/globals.css';
import './i18n';

type PageType = 'home' | 'settings' | 'gateway' | 'about';

function App() {
    const { initializeApp, config } = useAppStore();
    const [currentPage, setCurrentPage] = useState<PageType>('home');
    const [searchQuery, setSearchQuery] = useState('');
    const [triggerUpdateCheck, setTriggerUpdateCheck] = useState(false);
    const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);

    useEffect(() => {
        initializeApp();
    }, []);

    // Apply dark mode class to HTML element
    useEffect(() => {
        if (config?.theme === 'dark') {
            document.documentElement.classList.add('dark');
        } else {
            document.documentElement.classList.remove('dark');
        }
    }, [config?.theme]);

    const handleCheckUpdate = () => {
        setIsCheckingUpdate(true);
        setTriggerUpdateCheck(true);
    };

    const handleUpdateCheckComplete = () => {
        setTriggerUpdateCheck(false);
        setIsCheckingUpdate(false);
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
                onNavigate={setCurrentPage}
                onCheckUpdate={handleCheckUpdate}
                isCheckingUpdate={isCheckingUpdate}
            >
                {currentPage === 'home' && <Home searchQuery={searchQuery} />}
                {currentPage === 'settings' && <Settings />}
                {currentPage === 'gateway' && <Gateway />}
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


