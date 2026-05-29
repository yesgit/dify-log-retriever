import { useState } from 'react';
import type { Page } from './types';
import { Layout } from './components/Layout';
import { ConfigPage } from './components/ConfigPage';
import { AppsPage } from './components/AppsPage';
import { SyncPage } from './components/SyncPage';
import { ConversationsPage } from './components/ConversationsPage';
import { DashboardPage } from './components/DashboardPage';
import { ExportPage } from './components/ExportPage';

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('config');

  const renderPage = () => {
    switch (currentPage) {
      case 'config':
        return <ConfigPage />;
      case 'apps':
        return <AppsPage />;
      case 'sync':
        return <SyncPage />;
      case 'conversations':
        return <ConversationsPage />;
      case 'dashboard':
        return <DashboardPage />;
      case 'export':
        return <ExportPage />;
      default:
        return <ConfigPage />;
    }
  };

  return (
    <Layout currentPage={currentPage} onNavigate={setCurrentPage}>
      {renderPage()}
    </Layout>
  );
}

export default App;