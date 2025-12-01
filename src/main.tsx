import React, { useEffect, useState } from 'react';
import ReactDOM from 'react-dom/client';
import { Layout } from '@/components/Layout';
import { Home } from '@/pages/Home';
import { Settings } from '@/pages/Settings';
import { Gateway } from '@/pages/Gateway';
import { useAppStore } from '@/stores/appStore';
import '@/styles/globals.css';
import './i18n';

function App() {
    const { initializeApp, config } = useAppStore();
    const [currentPage, setCurrentPage] = useState<'home' | 'settings' | 'gateway'>('home');
    const [searchQuery, setSearchQuery] = useState('');

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

    return (
        <Layout
            onSearch={setSearchQuery}
            currentPage={currentPage}
            onNavigate={setCurrentPage}
        >
            {currentPage === 'home' && <Home searchQuery={searchQuery} />}
            {currentPage === 'settings' && <Settings />}
            {currentPage === 'gateway' && <Gateway />}
        </Layout>
    );
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
        <App />
    </React.StrictMode>
);
