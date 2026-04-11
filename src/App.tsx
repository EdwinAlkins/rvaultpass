import { useEffect, useMemo } from 'preact/hooks';
import { useTranslation } from 'react-i18next';
import { useAppStore } from './stores/app';
import type { VaultEntry } from './types';

import VaultScreen from './components/VaultScreen';
import Sidebar from './components/Sidebar';
import TopBar from './components/TopBar';
import SearchBar from './components/SearchBar';
import EntryList from './components/EntryList';
import EntryForm from './components/EntryForm';
import PasswordGenerator from './components/PasswordGenerator';
import Notifications from './components/Notifications';
import TOTPDisplay from './components/TOTPDisplay';

import './App.css';

function App() {
  const { t, i18n } = useTranslation();
  const {
    vaultOpen, darkMode, language,
    entries, folders, tags,
    selectedEntryId, viewFilter, selectedFolderId, selectedTagId,
    showEntryForm, editingEntry, startNewEntry, startEditEntry, closeEntryForm,
    showGenerator, toggleGenerator,
    searchResults, refreshData,
    setSelectedEntry,
  } = useAppStore();

  // Sync language
  useEffect(() => {
    if (i18n.language !== language) {
      i18n.changeLanguage(language);
    }
  }, [language, i18n]);

  // Apply theme
  useEffect(() => {
    document.documentElement.classList.toggle('light', !darkMode);
  }, [darkMode]);

  // Filtered entries
  const filteredEntries = useMemo(() => {
    let result = entries;

    if (searchResults !== null) {
      return searchResults;
    }

    switch (viewFilter) {
      case 'favorites':
        result = result.filter((e) => e.is_favorite);
        break;
      case 'folder':
        result = result.filter((e) => e.folder_id === selectedFolderId);
        break;
      case 'tag':
        result = result.filter((e) => e.tags.some((tag) => tag.id === selectedTagId));
        break;
    }

    return result;
  }, [entries, viewFilter, selectedFolderId, selectedTagId, searchResults]);

  // Selected entry for detail panel
  const selectedEntry = useMemo(
    () => entries.find((e) => e.id === selectedEntryId) ?? null,
    [entries, selectedEntryId]
  );

  const handleEntryClick = (entry: VaultEntry) => {
    setSelectedEntry(entry.id);
  };

  if (!vaultOpen) {
    return (
      <>
        <VaultScreen />
        <Notifications />
      </>
    );
  }

  return (
    <div class="app-layout" data-theme={darkMode ? 'dark' : 'light'}>
      <Sidebar />

      <div class="main-content">
        <TopBar onAddEntry={startNewEntry} onToggleGenerator={toggleGenerator} />
        <SearchBar />

        <div class="content-area">
          <EntryList entries={filteredEntries} onEntryClick={handleEntryClick} />
        </div>

        {/* Entry detail panel when selected */}
        {selectedEntry && (
          <div class="detail-panel">
            <div class="detail-header">
              <h2>{selectedEntry.title}</h2>
              <div class="detail-actions">
                <button class="detail-btn" onClick={() => startEditEntry(selectedEntry)}>
                  {t('common.edit')}
                </button>
                <button
                  class="detail-btn danger"
                  onClick={async () => {
                    if (window.confirm(t('entry.delete_confirm'))) {
                      const { deleteEntry } = await import('./api');
                      await deleteEntry(selectedEntry.id);
                      setSelectedEntry(null);
                      await refreshData();
                    }
                  }}
                >
                  {t('common.delete')}
                </button>
                <button class="detail-btn" onClick={() => setSelectedEntry(null)}>
                  ✕
                </button>
              </div>
            </div>

            <div class="detail-body">
              {selectedEntry.username && (
                <DetailField
                  label={t('entry.username')}
                  value={selectedEntry.username}
                  copyable
                />
              )}
              {selectedEntry.url && (
                <DetailField
                  label={t('entry.url')}
                  value={selectedEntry.url}
                  copyable
                  link
                />
              )}
              {selectedEntry.password && (
                <DetailField
                  label={t('entry.password')}
                  value="••••••••"
                  rawValue={selectedEntry.password}
                  copyable
                />
              )}
              {selectedEntry.notes && (
                <DetailField
                  label={t('entry.notes')}
                  value={selectedEntry.notes}
                  multiline
                />
              )}
              {selectedEntry.totp_secret && (
                <TOTPDisplay secret={selectedEntry.totp_secret} />
              )}
            </div>
          </div>
        )}
      </div>

      {/* Entry Form Modal */}
      {showEntryForm && (
        <EntryForm
          entry={editingEntry}
          folders={folders}
          allTags={tags}
          onClose={closeEntryForm}
          onSaved={async () => {
            closeEntryForm();
            setSelectedEntry(null);
            await refreshData();
          }}
        />
      )}

      {/* Password Generator Modal */}
      {showGenerator && (
        <PasswordGenerator onClose={toggleGenerator} />
      )}

      <Notifications />
    </div>
  );
}

// Detail field sub-component
interface DetailFieldProps {
  label: string;
  value: string;
  rawValue?: string;
  copyable?: boolean;
  link?: boolean;
  multiline?: boolean;
}

function DetailField({ label, value, rawValue, copyable, link, multiline }: DetailFieldProps) {
  const { addNotification } = useAppStore();

  const handleCopy = () => {
    const text = rawValue ?? value;
    navigator.clipboard.writeText(text);
    addNotification('Copied to clipboard', 'info');
  };

  return (
    <div class="detail-field">
      <label class="detail-label">{label}</label>
      <div class="detail-value">
        {link ? (
          <a href={value} target="_blank" rel="noopener noreferrer" class="detail-link">
            {value}
          </a>
        ) : multiline ? (
          <pre>{value}</pre>
        ) : (
          <span>{value}</span>
        )}
        {copyable && rawValue && (
          <button class="detail-copy-btn" onClick={handleCopy}>
            <svg viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
              <path d="M8 3a1 1 0 011-1h2a1 1 0 110 2H9a1 1 0 01-1-1z"/>
              <path d="M6 3a2 2 0 00-2 2v11a2 2 0 002 2h8a2 2 0 002-2V5a2 2 0 00-2-2 3 3 0 01-3 3H9a3 3 0 01-3-3z"/>
            </svg>
          </button>
        )}
      </div>
    </div>
  );
}

export default App;
