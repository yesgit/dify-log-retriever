import { useState, useEffect, useRef, useCallback } from 'react';
import type { Page, AutoSyncSettings } from './types';
import { invoke } from '@tauri-apps/api/core';
import { Layout } from './components/Layout';
import { ConfigPage } from './components/ConfigPage';
import { AppsPage } from './components/AppsPage';
import { SyncPage } from './components/SyncPage';
import { ConversationsPage } from './components/ConversationsPage';
import { FeedbackPage } from './components/FeedbackPage';
import { DashboardPage } from './components/DashboardPage';
import { PerformancePage } from './components/PerformancePage';
import { ExportPage } from './components/ExportPage';

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('config');
  const [autoSyncStatus, setAutoSyncStatus] = useState<'idle' | 'syncing' | 'success' | 'error'>('idle');
  const [lastAutoSyncResult, setLastAutoSyncResult] = useState<string>('');
  const autoSyncSettingsRef = useRef<AutoSyncSettings>({
    enabled: false,
    interval_minutes: 30,
    mode: 'incremental',
    last_synced_at: null,
  });
  const autoSyncTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const isAutoSyncingRef = useRef(false);
  const statusTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const doAutoSync = useCallback(async () => {
    if (isAutoSyncingRef.current) return;
    isAutoSyncingRef.current = true;
    setAutoSyncStatus('syncing');

    // Clear any previous status timeout
    if (statusTimeoutRef.current) {
      clearTimeout(statusTimeoutRef.current);
      statusTimeoutRef.current = null;
    }

    try {
      const settings = autoSyncSettingsRef.current;
      const result = await invoke<string>('sync_all_apps', {
        incremental: settings.mode === 'incremental',
      });
      setAutoSyncStatus('success');
      setLastAutoSyncResult(result);
      // Trigger dashboard aggregation after successful sync
      try { await invoke<string>('rebuild_dashboard_stats'); } catch (e) { console.error('Auto agg failed:', e); }
      // Reload settings to get updated last_synced_at
      const newSettings = await invoke<AutoSyncSettings>('get_auto_sync_settings');
      if (newSettings) {
        autoSyncSettingsRef.current = newSettings;
      }
    } catch (e) {
      console.error('Auto sync failed:', e);
      setAutoSyncStatus('error');
      setLastAutoSyncResult(`自动同步失败: ${e}`);
    } finally {
      isAutoSyncingRef.current = false;
      // Reset status after 30 seconds
      statusTimeoutRef.current = setTimeout(() => {
        setAutoSyncStatus((prev) => (prev === 'success' || prev === 'error') ? 'idle' : prev);
        setLastAutoSyncResult('');
        statusTimeoutRef.current = null;
      }, 30000);
    }
  }, []);

  // Setup auto sync timer
  const setupAutoSyncTimer = useCallback(() => {
    // Clear existing timer
    if (autoSyncTimerRef.current) {
      clearInterval(autoSyncTimerRef.current);
      autoSyncTimerRef.current = null;
    }

    const settings = autoSyncSettingsRef.current;

    if (!settings.enabled) return;

    const intervalMs = settings.interval_minutes * 60 * 1000;

    // Check if we should sync immediately
    if (settings.last_synced_at) {
      const lastSync = settings.last_synced_at * 1000;
      const elapsed = Date.now() - lastSync;
      if (elapsed >= intervalMs) {
        doAutoSync();
      }
    } else {
      // Never synced before, sync immediately
      doAutoSync();
    }

    // Set up periodic timer (check every minute if it's time to sync)
    autoSyncTimerRef.current = setInterval(() => {
      const currentSettings = autoSyncSettingsRef.current;
      if (!currentSettings.enabled) return;

      const interval = currentSettings.interval_minutes * 60 * 1000;
      const lastSync = currentSettings.last_synced_at
        ? currentSettings.last_synced_at * 1000
        : 0;

      if (Date.now() - lastSync >= interval) {
        doAutoSync();
      }
    }, 60000); // Check every minute
  }, [doAutoSync]);

  // Initial setup
  useEffect(() => {
    const init = async () => {
      try {
        const settings = await invoke<AutoSyncSettings>('get_auto_sync_settings');
        if (settings) {
          autoSyncSettingsRef.current = settings;
        }
      } catch (e) {
        console.error(e);
      }
      setupAutoSyncTimer();
    };
    init();

    return () => {
      if (autoSyncTimerRef.current) {
        clearInterval(autoSyncTimerRef.current);
      }
      if (statusTimeoutRef.current) {
        clearTimeout(statusTimeoutRef.current);
      }
    };
  }, [setupAutoSyncTimer]);

  // Reload settings periodically so changes in SyncPage are picked up
  useEffect(() => {
    const reloadSettings = async () => {
      try {
        const settings = await invoke<AutoSyncSettings>('get_auto_sync_settings');
        if (settings) {
          const prevEnabled = autoSyncSettingsRef.current.enabled;
          const prevInterval = autoSyncSettingsRef.current.interval_minutes;
          autoSyncSettingsRef.current = settings;

          // If settings changed, restart the timer
          if (prevEnabled !== settings.enabled || prevInterval !== settings.interval_minutes) {
            setupAutoSyncTimer();
          }
        }
      } catch (e) {
        console.error(e);
      }
    };

    const settingsReloadTimer = setInterval(reloadSettings, 15000); // Every 15 seconds
    return () => clearInterval(settingsReloadTimer);
  }, [setupAutoSyncTimer]);

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
      case 'feedback':
        return <FeedbackPage />;
      case 'dashboard':
        return <DashboardPage />;
      case 'performance':
        return <PerformancePage />;
      case 'export':
        return <ExportPage />;
      default:
        return <ConfigPage />;
    }
  };

  return (
    <Layout
      currentPage={currentPage}
      onNavigate={setCurrentPage}
      autoSyncStatus={autoSyncStatus}
      lastAutoSyncResult={lastAutoSyncResult}
    >
      {renderPage()}
    </Layout>
  );
}

export default App;