import { useTranslation } from 'react-i18next';
import { useAppStore } from '../stores/app';
import * as api from '../api';
import { open } from '@tauri-apps/plugin-dialog';
import './TopBar.css';

interface TopBarProps {
  onAddEntry: () => void;
  onToggleGenerator: () => void;
}

export default function TopBar({ onAddEntry, onToggleGenerator }: TopBarProps) {
  const { t } = useTranslation();
  const {
    darkMode, toggleDarkMode, language, setLanguage,
    clearVaultState, addNotification, viewFilter, refreshData,
  } = useAppStore();

  const handleLock = async () => {
    try {
      await api.lockVault();
      clearVaultState();
      addNotification(t('notif.vault_locked'), 'success');
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      addNotification(msg, 'error');
    }
  };

  const handleSave = async () => {
    try {
      await api.saveVault();
      addNotification(t('notif.vault_saved'), 'success');
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      addNotification(msg, 'error');
    }
  };

  const handleImportDashlane = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (!selected || Array.isArray(selected)) {
        return;
      }

      const summary = await api.importDashlaneCsv(selected);
      await refreshData();

      if (summary.skipped > 0 || summary.errors.length > 0) {
        addNotification(
          t('notif.import_partial', {
            imported: summary.imported,
            skipped: summary.skipped,
          }),
          summary.imported > 0 ? 'info' : 'error',
        );
      } else {
        addNotification(
          t('notif.import_done', { count: summary.imported }),
          'success',
        );
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : t('notif.import_failed');
      addNotification(msg, 'error');
    }
  };

  const title = viewFilter === 'favorites'
    ? t('sidebar.favorites')
    : viewFilter === 'all'
    ? t('sidebar.entries')
    : viewFilter === 'folder'
    ? t('sidebar.folders')
    : t('sidebar.tags');

  return (
    <header class="topbar" data-theme={darkMode ? 'dark' : 'light'}>
      <div class="topbar-left">
        <h1 class="topbar-title">{title}</h1>
      </div>

      <div class="topbar-actions">
        <button class="action-btn" onClick={onToggleGenerator} title={t('generator.title')}>
          <svg viewBox="0 0 20 20" fill="currentColor" width="18" height="18">
            <path fillRule="evenodd" d="M11.3 1.046A1 1 0 0112 2v5h4a1 1 0 01.82 1.573l-7 10A1 1 0 018 18v-5H4a1 1 0 01-.82-1.573l7-10a1 1 0 011.12-.38z" clipRule="evenodd"/>
          </svg>
        </button>

        <button class="action-btn" onClick={onAddEntry} title={t('entry.add')}>
          <svg viewBox="0 0 20 20" fill="currentColor" width="18" height="18">
            <path fillRule="evenodd" d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" clipRule="evenodd"/>
          </svg>
        </button>

        <button class="action-btn" onClick={handleImportDashlane} title={t('action.import_dashlane')}>
          <svg viewBox="0 0 20 20" fill="currentColor" width="18" height="18">
            <path d="M10.75 2.75a.75.75 0 00-1.5 0v8.614L6.295 8.235a.75.75 0 10-1.09 1.03l4.25 4.5a.75.75 0 001.09 0l4.25-4.5a.75.75 0 00-1.09-1.03l-2.955 3.129V2.75z"/>
            <path d="M3.5 12.75a.75.75 0 00-1.5 0v2.5A2.75 2.75 0 004.75 18h10.5A2.75 2.75 0 0018 15.25v-2.5a.75.75 0 00-1.5 0v2.5c0 .69-.56 1.25-1.25 1.25H4.75c-.69 0-1.25-.56-1.25-1.25v-2.5z"/>
          </svg>
        </button>

        <button class="action-btn" onClick={handleSave} title={t('action.save')}>
          <svg viewBox="0 0 20 20" fill="currentColor" width="18" height="18">
            <path d="M7.707 10.293a1 1 0 10-1.414 1.414l3 3a1 1 0 001.414 0l3-3a1 1 0 00-1.414-1.414L11 11.586V6h5a2 2 0 012 2v7a2 2 0 01-2 2H4a2 2 0 01-2-2V8a2 2 0 012-2h5v5.586l-1.293-1.293zM9 4a1 1 0 012 0v2H9V4z"/>
          </svg>
        </button>

        <button class="action-btn" onClick={handleLock} title={t('action.lock')}>
          <svg viewBox="0 0 20 20" fill="currentColor" width="18" height="18">
            <path fillRule="evenodd" d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z" clipRule="evenodd"/>
          </svg>
        </button>

        <div class="divider" />

        <button class="action-btn" onClick={toggleDarkMode} title={darkMode ? t('theme.light') : t('theme.dark')}>
          {darkMode ? (
            <svg viewBox="0 0 20 20" fill="currentColor" width="18" height="18">
              <path fillRule="evenodd" d="M10 2a1 1 0 011 1v1a1 1 0 11-2 0V3a1 1 0 011-1zm4 8a4 4 0 11-8 0 4 4 0 018 0zm-.464 4.95l.707.707a1 1 0 001.414-1.414l-.707-.707a1 1 0 00-1.414 1.414zm2.12-10.607a1 1 0 010 1.414l-.706.707a1 1 0 11-1.414-1.414l.707-.707a1 1 0 011.414 0zM17 11a1 1 0 100-2h-1a1 1 0 100 2h1zm-7 4a1 1 0 011 1v1a1 1 0 11-2 0v-1a1 1 0 011-1zM5.05 6.464A1 1 0 106.465 5.05l-.708-.707a1 1 0 00-1.414 1.414l.707.707zm1.414 8.486l-.707.707a1 1 0 01-1.414-1.414l.707-.707a1 1 0 011.414 1.414zM4 11a1 1 0 100-2H3a1 1 0 000 2h1z" clipRule="evenodd"/>
            </svg>
          ) : (
            <svg viewBox="0 0 20 20" fill="currentColor" width="18" height="18">
              <path d="M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z"/>
            </svg>
          )}
        </button>

        <button
          class="lang-btn"
          onClick={() => setLanguage(language === 'en' ? 'fr' : 'en')}
        >
          {language.toUpperCase()}
        </button>
      </div>
    </header>
  );
}
