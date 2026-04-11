import { useState } from 'preact/hooks';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../stores/app';
import * as api from '../api';
import './Sidebar.css';

export default function Sidebar() {
  const { t } = useTranslation();
  const {
    folders, tags, viewFilter, selectedFolderId, selectedTagId,
    setViewFilter, setSelectedFolder, setSelectedTag,
    refreshData, addNotification, darkMode,
  } = useAppStore();

  const [addingFolder, setAddingFolder] = useState(false);
  const [addingTag, setAddingTag] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const [newTagName, setNewTagName] = useState('');

  const handleAddFolder = async () => {
    if (!newFolderName.trim()) return;
    try {
      await api.createFolder(newFolderName.trim(), null);
      setNewFolderName('');
      setAddingFolder(false);
      await refreshData();
      addNotification(t('notif.folder_created'), 'success');
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      addNotification(msg, 'error');
    }
  };

  const handleAddTag = async () => {
    if (!newTagName.trim()) return;
    try {
      await api.createTag(newTagName.trim());
      setNewTagName('');
      setAddingTag(false);
      await refreshData();
      addNotification(t('notif.tag_created'), 'success');
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      addNotification(msg, 'error');
    }
  };

  return (
    <aside class="sidebar" data-theme={darkMode ? 'dark' : 'light'}>
      <div class="sidebar-header">
        <svg viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg" class="logo-icon">
          <rect x="6" y="10" width="20" height="16" rx="2" stroke="currentColor" strokeWidth="2"/>
          <path d="M11 10V8C11 5.239 13.239 3 16 3C18.761 3 21 5.239 21 8V10" stroke="currentColor" strokeWidth="2"/>
          <circle cx="16" cy="19" r="2" fill="currentColor"/>
        </svg>
        <span class="app-name">RVaultPass</span>
      </div>

      <nav class="sidebar-nav">
        <button
          class={`nav-item ${viewFilter === 'all' ? 'active' : ''}`}
          onClick={() => setViewFilter('all')}
        >
          <svg viewBox="0 0 20 20" fill="currentColor" class="nav-icon">
            <path d="M3 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1V4zm0 6a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1v-2zm0 6a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-1 1H4a1 1 0 01-1-1v-2z"/>
          </svg>
          {t('sidebar.entries')}
        </button>

        <button
          class={`nav-item ${viewFilter === 'favorites' ? 'active' : ''}`}
          onClick={() => setViewFilter('favorites')}
        >
          <svg viewBox="0 0 20 20" fill="currentColor" class="nav-icon">
            <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z"/>
          </svg>
          {t('sidebar.favorites')}
        </button>
      </nav>

      {/* Folders */}
      <div class="sidebar-section">
        <div class="section-header">
          <span>{t('sidebar.folders')}</span>
          <button class="add-btn" onClick={() => setAddingFolder(true)}>+</button>
        </div>

        {addingFolder && (
          <div class="inline-input animate-slide-in">
            <input
              type="text"
              value={newFolderName}
              onInput={(e) => setNewFolderName(e.currentTarget.value)}
              placeholder={t('sidebar.new_folder_name')}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAddFolder();
                if (e.key === 'Escape') { setAddingFolder(false); setNewFolderName(''); }
              }}
              autoFocus
            />
            <div class="inline-actions">
              <button onClick={handleAddFolder} class="confirm-btn">✓</button>
              <button onClick={() => { setAddingFolder(false); setNewFolderName(''); }}>✕</button>
            </div>
          </div>
        )}

        <div class="folder-list">
          {folders.map((folder) => (
            <button
              class={`nav-item sub-item ${viewFilter === 'folder' && selectedFolderId === folder.id ? 'active' : ''}`}
              onClick={() => {
                setViewFilter('folder', folder.id);
                setSelectedFolder(folder.id);
              }}
            >
              <svg viewBox="0 0 20 20" fill="currentColor" class="nav-icon">
                <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"/>
              </svg>
              {folder.name}
            </button>
          ))}
        </div>
      </div>

      {/* Tags */}
      <div class="sidebar-section">
        <div class="section-header">
          <span>{t('sidebar.tags')}</span>
          <button class="add-btn" onClick={() => setAddingTag(true)}>+</button>
        </div>

        {addingTag && (
          <div class="inline-input animate-slide-in">
            <input
              type="text"
              value={newTagName}
              onInput={(e) => setNewTagName(e.currentTarget.value)}
              placeholder={t('sidebar.new_tag_name')}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAddTag();
                if (e.key === 'Escape') { setAddingTag(false); setNewTagName(''); }
              }}
              autoFocus
            />
            <div class="inline-actions">
              <button onClick={handleAddTag} class="confirm-btn">✓</button>
              <button onClick={() => { setAddingTag(false); setNewTagName(''); }}>✕</button>
            </div>
          </div>
        )}

        <div class="tag-list">
          {tags.map((tag) => (
            <button
              class={`tag-item ${viewFilter === 'tag' && selectedTagId === tag.id ? 'active' : ''}`}
              onClick={() => {
                setViewFilter('tag', tag.id);
                setSelectedTag(tag.id);
              }}
            >
              <span class="tag-dot" />
              {tag.name}
            </button>
          ))}
        </div>
      </div>
    </aside>
  );
}
