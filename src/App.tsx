import { useEffect, useMemo, useState } from 'preact/hooks';
import { useTranslation } from 'react-i18next';
import { useAppStore } from './stores/app';
import type { EntrySummary } from './types';
import * as api from './api';

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

  const [revealedPassword, setRevealedPassword] = useState<string | null>(null);
  const [revealedNotes, setRevealedNotes] = useState<string | null>(null);

  useEffect(() => {
    if (i18n.language !== language) {
      i18n.changeLanguage(language);
    }
  }, [language, i18n]);

  useEffect(() => {
    document.documentElement.classList.toggle('light', !darkMode);
  }, [darkMode]);

  // Clear ephemeral reveals when selection changes
  useEffect(() => {
    setRevealedPassword(null);
    setRevealedNotes(null);
  }, [selectedEntryId]);

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

  const selectedEntry = useMemo(
    () => entries.find((e) => e.id === selectedEntryId) ?? null,
    [entries, selectedEntryId]
  );

  const handleEntryClick = (entry: EntrySummary) => {
    setSelectedEntry(entry.id);
  };

  const handleRevealPassword = async () => {
    if (!selectedEntry) return;
    try {
      const pw = await api.revealPassword(selectedEntry.id);
      setRevealedPassword(pw);
      // Auto-hide after 15s
      setTimeout(() => setRevealedPassword(null), 15000);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      useAppStore.getState().addNotification(msg, 'error');
    }
  };

  const handleCopyField = async (field: string) => {
    if (!selectedEntry) return;
    try {
      await api.copyEntryField(selectedEntry.id, field);
      useAppStore.getState().addNotification(
        t('notif.copied_clipboard') + ' — ' + t('notif.clipboard_warning'),
        'info',
      );
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      useAppStore.getState().addNotification(msg, 'error');
    }
  };

  const handleRevealNotes = async () => {
    if (!selectedEntry) return;
    try {
      // Notes are fetched via edit endpoint (no password/totp)
      const data = await api.getEntry(selectedEntry.id);
      setRevealedNotes(data.notes ?? '');
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      useAppStore.getState().addNotification(msg, 'error');
    }
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

        {selectedEntry && (
          <div class="detail-panel">
            <div class="detail-header">
              <h2>{selectedEntry.title}</h2>
              <div class="detail-actions">
                <button class="detail-btn" onClick={() => startEditEntry(selectedEntry.id)}>
                  {t('common.edit')}
                </button>
                <button
                  class="detail-btn danger"
                  onClick={async () => {
                    if (window.confirm(t('entry.delete_confirm'))) {
                      await api.deleteEntry(selectedEntry.id);
                      await api.saveVault();
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
                  onCopy={() => handleCopyField('username')}
                />
              )}
              {selectedEntry.username2 && (
                <DetailField
                  label={t('entry.username2')}
                  value={selectedEntry.username2}
                  onCopy={() => handleCopyField('username2')}
                />
              )}
              {selectedEntry.username3 && (
                <DetailField
                  label={t('entry.username3')}
                  value={selectedEntry.username3}
                  onCopy={() => handleCopyField('username3')}
                />
              )}
              {selectedEntry.url && (
                <DetailField
                  label={t('entry.url')}
                  value={selectedEntry.url}
                  onCopy={() => handleCopyField('url')}
                  link
                />
              )}
              {selectedEntry.has_password && (
                <DetailField
                  label={t('entry.password')}
                  value={revealedPassword ?? '••••••••'}
                  onCopy={() => handleCopyField('password')}
                  onReveal={!revealedPassword ? handleRevealPassword : undefined}
                />
              )}
              {selectedEntry.has_notes && (
                revealedNotes !== null ? (
                  <DetailField
                    label={t('entry.notes')}
                    value={revealedNotes}
                    multiline
                    onCopy={() => handleCopyField('notes')}
                  />
                ) : (
                  <div class="detail-field">
                    <label class="detail-label">{t('entry.notes')}</label>
                    <button class="detail-btn" onClick={handleRevealNotes}>
                      {t('entry.reveal_notes')}
                    </button>
                  </div>
                )
              )}
              {selectedEntry.has_totp && (
                <TOTPDisplay entryId={selectedEntry.id} />
              )}
            </div>
          </div>
        )}
      </div>

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

      {showGenerator && (
        <PasswordGenerator onClose={toggleGenerator} />
      )}

      <Notifications />
    </div>
  );
}

interface DetailFieldProps {
  label: string;
  value: string;
  onCopy?: () => void;
  onReveal?: () => void;
  link?: boolean;
  multiline?: boolean;
}

function DetailField({ label, value, onCopy, onReveal, link, multiline }: DetailFieldProps) {
  const { t } = useTranslation();

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
        {onReveal && (
          <button class="detail-copy-btn" onClick={onReveal} title={t('entry.reveal')}>
            👁
          </button>
        )}
        {onCopy && (
          <button class="detail-copy-btn" onClick={onCopy} title={t('common.copy')}>
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
