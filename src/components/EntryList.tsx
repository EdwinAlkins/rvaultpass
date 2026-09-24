import { useTranslation } from 'react-i18next';
import { useAppStore } from '../stores/app';
import type { EntrySummary } from '../types';
import './EntryList.css';

interface EntryListProps {
  entries: EntrySummary[];
  onEntryClick: (entry: EntrySummary) => void;
}

export default function EntryList({ entries, onEntryClick }: EntryListProps) {
  const { t } = useTranslation();
  const { darkMode } = useAppStore();

  if (entries.length === 0) {
    return (
      <div class="entry-list-empty" data-theme={darkMode ? 'dark' : 'light'}>
        <svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg" class="empty-icon">
          <rect x="16" y="8" width="32" height="48" rx="4" stroke="currentColor" strokeWidth="2"/>
          <line x1="24" y1="20" x2="40" y2="20" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
          <line x1="24" y1="28" x2="40" y2="28" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
          <line x1="24" y1="36" x2="34" y2="36" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
        </svg>
        <h3>{t('entry.list.empty')}</h3>
        <p>{t('entry.list.empty_desc')}</p>
      </div>
    );
  }

  return (
    <div class="entry-list" data-theme={darkMode ? 'dark' : 'light'}>
      {entries.map((entry) => (
        <div
          key={entry.id}
          class="entry-card"
          onClick={() => onEntryClick(entry)}
        >
          <div class="entry-header">
            <h3 class="entry-title">{entry.title || 'Untitled'}</h3>
            {entry.is_favorite && (
              <svg class="fav-star" viewBox="0 0 20 20" fill="currentColor">
                <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z"/>
              </svg>
            )}
          </div>
          {entry.username && (
            <p class="entry-username">{entry.username}</p>
          )}
          {entry.url && (
            <p class="entry-url">{entry.url}</p>
          )}
          <div class="entry-footer">
            <div class="entry-tags">
              {entry.tags.map((tag) => (
                <span class="entry-tag" key={tag.id}>{tag.name}</span>
              ))}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
