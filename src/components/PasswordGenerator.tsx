import { useState } from 'preact/hooks';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../stores/app';
import * as api from '../api';
import type { PasswordGenerationParams } from '../types';
import './PasswordGenerator.css';

interface PasswordGeneratorProps {
  onClose: () => void;
}

export default function PasswordGenerator({ onClose }: PasswordGeneratorProps) {
  const { t } = useTranslation();
  const { darkMode, addNotification } = useAppStore();

  const [mode, setMode] = useState<'password' | 'passphrase'>('password');
  const [params, setParams] = useState<PasswordGenerationParams>({
    length: 20,
    use_uppercase: true,
    use_lowercase: true,
    use_numbers: true,
    use_symbols: false,
    exclude_ambiguous: false,
  });
  const [wordCount, setWordCount] = useState(6);
  const [generated, setGenerated] = useState('');

  const handleGenerate = async () => {
    try {
      let result: string;
      if (mode === 'password') {
        result = await api.generatePassword(params);
      } else {
        result = await api.generatePassphrase(wordCount);
      }
      setGenerated(result);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      addNotification(msg, 'error');
    }
  };

  const handleCopy = async () => {
    if (generated) {
      try {
        await api.copyToClipboard(generated);
        addNotification(t('notif.copied_clipboard') + ' — ' + t('notif.clipboard_warning'), 'info');
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : 'Error';
        addNotification(msg, 'error');
      }
    }
  };

  const updateParam = (key: keyof PasswordGenerationParams, value: boolean | number) => {
    setParams((prev) => ({ ...prev, [key]: value }));
  };

  return (
    <div class="generator-overlay" data-theme={darkMode ? 'dark' : 'light'}>
      <div class="generator-panel animate-fade-in">
        <div class="gen-header">
          <h2>{t('generator.title')}</h2>
          <button class="close-btn" onClick={onClose}>✕</button>
        </div>

        <div class="gen-body">
          {/* Mode toggle */}
          <div class="mode-toggle">
            <button
              class={mode === 'password' ? 'active' : ''}
              onClick={() => setMode('password')}
            >
              {t('generator.title')}
            </button>
            <button
              class={mode === 'passphrase' ? 'active' : ''}
              onClick={() => setMode('passphrase')}
            >
              {t('generator.passphrase')}
            </button>
          </div>

          {mode === 'password' ? (
            <>
              <div class="gen-field">
                <label>
                  {t('generator.length')}: <strong>{params.length}</strong>
                </label>
                <input
                  type="range"
                  min="8"
                  max="64"
                  value={params.length}
                  onInput={(e) => updateParam('length', parseInt(e.currentTarget.value))}
                />
              </div>

              <div class="gen-checkboxes">
                <label class="check-item">
                  <input
                    type="checkbox"
                    checked={params.use_uppercase}
                    onChange={() => updateParam('use_uppercase', !params.use_uppercase)}
                  />
                  {t('generator.uppercase')}
                </label>
                <label class="check-item">
                  <input
                    type="checkbox"
                    checked={params.use_lowercase}
                    onChange={() => updateParam('use_lowercase', !params.use_lowercase)}
                  />
                  {t('generator.lowercase')}
                </label>
                <label class="check-item">
                  <input
                    type="checkbox"
                    checked={params.use_numbers}
                    onChange={() => updateParam('use_numbers', !params.use_numbers)}
                  />
                  {t('generator.numbers')}
                </label>
                <label class="check-item">
                  <input
                    type="checkbox"
                    checked={params.use_symbols}
                    onChange={() => updateParam('use_symbols', !params.use_symbols)}
                  />
                  {t('generator.symbols')}
                </label>
                <label class="check-item">
                  <input
                    type="checkbox"
                    checked={params.exclude_ambiguous}
                    onChange={() => updateParam('exclude_ambiguous', !params.exclude_ambiguous)}
                  />
                  {t('generator.exclude_ambiguous')}
                </label>
              </div>
            </>
          ) : (
            <div class="gen-field">
              <label>
                {t('generator.word_count')}: <strong>{wordCount}</strong>
              </label>
              <input
                type="range"
                min="3"
                max="12"
                value={wordCount}
                onInput={(e) => setWordCount(parseInt(e.currentTarget.value))}
              />
            </div>
          )}

          <button class="btn btn-primary btn-full" onClick={handleGenerate}>
            {mode === 'password' ? t('generator.generate') : t('generator.generate_passphrase')}
          </button>

          {generated && (
            <div class="gen-result animate-fade-in">
              <div class="result-text" onClick={handleCopy} title={t('common.copy')}>
                {generated}
              </div>
              <button class="btn btn-secondary copy-btn" onClick={handleCopy}>
                {t('common.copy')}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
