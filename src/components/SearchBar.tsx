import { useTranslation } from 'react-i18next';
import { useAppStore } from '../stores/app';
import * as api from '../api';
import './SearchBar.css';

export default function SearchBar() {
  const { t } = useTranslation();
  const { searchQuery, setSearchQuery, setSearchResults, darkMode } = useAppStore();

  const handleSearch = async (query: string) => {
    setSearchQuery(query);
    if (query.trim().length >= 2) {
      try {
        const results = await api.searchEntries(query.trim());
        setSearchResults(results);
      } catch {
        setSearchResults(null);
      }
    } else {
      setSearchResults(null);
    }
  };

  return (
    <div class="search-bar" data-theme={darkMode ? 'dark' : 'light'}>
      <svg viewBox="0 0 20 20" fill="currentColor" class="search-icon" width="16" height="16">
        <path fillRule="evenodd" d="M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z" clipRule="evenodd"/>
      </svg>
      <input
        type="text"
        value={searchQuery}
        onInput={(e) => handleSearch(e.currentTarget.value)}
        placeholder={t('common.search')}
        class="search-input"
      />
      {searchQuery && (
        <button class="clear-search" onClick={() => handleSearch('')}>
          ✕
        </button>
      )}
    </div>
  );
}
