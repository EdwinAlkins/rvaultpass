import { useState } from 'preact/hooks';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../stores/app';
import * as api from '../api';
import { open, save } from '@tauri-apps/plugin-dialog';
import './VaultScreen.css';

export default function VaultScreen() {
  const { t } = useTranslation();
  const [mode, setMode] = useState<'welcome' | 'create' | 'open'>('welcome');
  const [filePath, setFilePath] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const { setVaultOpen, addNotification, refreshData, darkMode, language, setLanguage } = useAppStore();

  const handleBrowse = async () => {
    try {
      if (mode === 'create') {
        // For create: use save dialog to let user choose location and filename
        const selected = await save({
          title: t('vault.create_title'),
          defaultPath: 'my-vault.vault',
          filters: [{
            name: 'Vault Files',
            extensions: ['vault'],
          }],
        });
        if (selected) {
          setFilePath(selected);
        }
      } else {
        // For open: select an existing .vault file
        const selected = await open({
          multiple: false,
          title: t('vault.open_title'),
          filters: [{
            name: 'Vault Files',
            extensions: ['vault'],
          }],
        });
        if (selected && typeof selected === 'string') {
          setFilePath(selected);
        } else if (selected && Array.isArray(selected) && selected.length > 0) {
          setFilePath(selected[0]);
        }
      }
    } catch (e) {
      console.error(e);
      const message = e instanceof Error ? e.message : 'Failed to browse files';
      setError(message);
    }
  };

  const handleSubmit = async () => {
    setError('');

    if (!filePath.trim()) {
      setError(t('vault.empty_path'));
      return;
    }
    if (!password) {
      setError(t('vault.empty_password'));
      return;
    }
    if (mode === 'create' && password !== confirmPassword) {
      setError(t('vault.passwords_mismatch'));
      return;
    }

    setLoading(true);
    try {
      if (mode === 'create') {
        await api.createVault(filePath, password);
        addNotification(t('common.success'), 'success');
      } else {
        await api.openVault(filePath, password);
      }
      setVaultOpen(true);
      await refreshData();
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'An error occurred';
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="vault-screen" data-theme={darkMode ? 'dark' : 'light'}>
      {/* Language selector */}
      <div class="lang-selector">
        <button
          class={language === 'en' ? 'active' : ''}
          onClick={() => setLanguage('en')}
        >
          EN
        </button>
        <button
          class={language === 'fr' ? 'active' : ''}
          onClick={() => setLanguage('fr')}
        >
          FR
        </button>
      </div>

      <div class="vault-container animate-fade-in">
        {mode === 'welcome' ? (
          <div class="welcome-screen">
            <div class="vault-icon">
              <svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
                <rect x="12" y="20" width="40" height="32" rx="4" stroke="currentColor" strokeWidth="3"/>
                <path d="M22 20V16C22 10.477 26.477 6 32 6C37.523 6 42 10.477 42 16V20" stroke="currentColor" strokeWidth="3"/>
                <circle cx="32" cy="38" r="4" fill="currentColor"/>
                <line x1="32" y1="42" x2="32" y2="46" stroke="currentColor" strokeWidth="3" strokeLinecap="round"/>
              </svg>
            </div>
            <h1>{t('vault.welcome')}</h1>
            <p>{t('vault.description')}</p>
            <div class="button-group">
              <button class="btn btn-primary" onClick={() => setMode('create')}>
                {t('vault.create')}
              </button>
              <button class="btn btn-secondary" onClick={() => setMode('open')}>
                {t('vault.open')}
              </button>
            </div>
          </div>
        ) : (
          <div class="form-screen animate-fade-in">
            <button class="back-btn" onClick={() => { setMode('welcome'); setError(''); }}>
              ← Back
            </button>
            <h1>{mode === 'create' ? t('vault.create') : t('vault.open')}</h1>

            {error && <div class="error-msg">{error}</div>}

            <div class="form-group">
              <label>{t('vault.file_path')}</label>
              <div class="file-path-input">
                <input
                  type="text"
                  value={filePath}
                  onInput={(e) => setFilePath(e.currentTarget.value)}
                  placeholder={t('vault.file_path_placeholder')}
                />
                <button
                  class="browse-btn"
                  type="button"
                  onClick={handleBrowse}
                  title={t('vault.browse')}
                >
                  📁
                </button>
              </div>
            </div>

            <div class="form-group">
              <label>{t('vault.password')}</label>
              <input
                type="password"
                value={password}
                onInput={(e) => setPassword(e.currentTarget.value)}
                placeholder={t('vault.password_placeholder')}
              />
            </div>

            {mode === 'create' && (
              <div class="form-group">
                <label>{t('vault.confirm_password')}</label>
                <input
                  type="password"
                  value={confirmPassword}
                  onInput={(e) => setConfirmPassword(e.currentTarget.value)}
                  placeholder={t('vault.confirm_password_placeholder')}
                />
              </div>
            )}

            <button
              class="btn btn-primary btn-full"
              onClick={handleSubmit}
              disabled={loading}
            >
              {loading
                ? (mode === 'create' ? t('vault.creating') : t('vault.opening'))
                : (mode === 'create' ? t('vault.create') : t('vault.open'))}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
