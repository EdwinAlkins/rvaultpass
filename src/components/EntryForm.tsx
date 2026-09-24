import { useState } from 'preact/hooks';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../stores/app';
import * as api from '../api';
import type { EntryEditData, VaultTag } from '../types';
import './EntryForm.css';

interface EntryFormProps {
  entry: EntryEditData | null;
  folders: { id: string; name: string }[];
  allTags: VaultTag[];
  onClose: () => void;
  onSaved: () => void;
}

export default function EntryForm({ entry, folders, allTags, onClose, onSaved }: EntryFormProps) {
  const { t } = useTranslation();
  const { darkMode, addNotification } = useAppStore();

  const [title, setTitle] = useState(entry?.title ?? '');
  const [url, setUrl] = useState(entry?.url ?? '');
  const [username, setUsername] = useState(entry?.username ?? '');
  const [username2, setUsername2] = useState(entry?.username2 ?? '');
  const [username3, setUsername3] = useState(entry?.username3 ?? '');
  const [password, setPassword] = useState('');
  const [notes, setNotes] = useState(entry?.notes ?? '');
  const [totpSecret, setTotpSecret] = useState('');
  const [folderId, setFolderId] = useState(entry?.folder_id ?? '');
  const [isFavorite, setIsFavorite] = useState(entry?.is_favorite ?? false);
  const [selectedTagIds, setSelectedTagIds] = useState<string[]>(
    entry?.tags?.map((tag) => tag.id) ?? []
  );
  const [showPassword, setShowPassword] = useState(false);
  const [saving, setSaving] = useState(false);

  const toggleTag = (tagId: string) => {
    setSelectedTagIds((prev) =>
      prev.includes(tagId) ? prev.filter((id) => id !== tagId) : [...prev, tagId]
    );
  };

  const handleSave = async () => {
    if (!title.trim()) return;
    setSaving(true);
    try {
      if (entry) {
        await api.updateEntry(
          entry.id,
          folderId || null,
          title.trim(),
          url.trim() || null,
          username.trim() || null,
          username2.trim() || null,
          username3.trim() || null,
          password || null, // empty = keep existing
          notes.trim() || null,
          totpSecret.trim() || null, // empty = keep existing
          isFavorite,
          selectedTagIds.length > 0 ? selectedTagIds : null,
        );
        addNotification(t('notif.entry_updated'), 'success');
      } else {
        await api.createEntry(
          folderId || null,
          title.trim(),
          url.trim() || null,
          username.trim() || null,
          username2.trim() || null,
          username3.trim() || null,
          password,
          notes.trim() || null,
          totpSecret.trim() || null,
          isFavorite,
          selectedTagIds.length > 0 ? selectedTagIds : null,
        );
        addNotification(t('notif.entry_created'), 'success');
      }
      await api.saveVault();
      onSaved();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      addNotification(msg, 'error');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="entry-form-overlay" data-theme={darkMode ? 'dark' : 'light'}>
      <div class="entry-form animate-fade-in">
        <div class="form-header">
          <h2>{entry ? t('entry.edit') : t('entry.add')}</h2>
          <button class="close-btn" onClick={onClose}>✕</button>
        </div>

        <div class="form-body">
          <div class="form-group">
            <label>{t('entry.title')}</label>
            <input
              type="text"
              value={title}
              onInput={(e) => setTitle(e.currentTarget.value)}
              placeholder={t('entry.title_placeholder')}
            />
          </div>

          <div class="form-group">
            <label>{t('entry.url')}</label>
            <input
              type="text"
              value={url}
              onInput={(e) => setUrl(e.currentTarget.value)}
              placeholder={t('entry.url_placeholder')}
            />
          </div>

          <div class="form-group">
            <label>{t('entry.username')}</label>
            <input
              type="text"
              value={username}
              onInput={(e) => setUsername(e.currentTarget.value)}
              placeholder={t('entry.username_placeholder')}
            />
          </div>

          <div class="form-group">
            <label>{t('entry.username2')}</label>
            <input
              type="text"
              value={username2}
              onInput={(e) => setUsername2(e.currentTarget.value)}
              placeholder={t('entry.username2_placeholder')}
            />
          </div>

          <div class="form-group">
            <label>{t('entry.username3')}</label>
            <input
              type="text"
              value={username3}
              onInput={(e) => setUsername3(e.currentTarget.value)}
              placeholder={t('entry.username3_placeholder')}
            />
          </div>

          <div class="form-group">
            <label>
              {t('entry.password')}
              {entry?.has_password ? ` (${t('entry.password_keep_hint')})` : ''}
            </label>
            <div class="password-field">
              <input
                type={showPassword ? 'text' : 'password'}
                value={password}
                onInput={(e) => setPassword(e.currentTarget.value)}
                placeholder={entry?.has_password ? '••••••••' : ''}
              />
              <button class="toggle-vis" onClick={() => setShowPassword(!showPassword)}>
                {showPassword ? '🙈' : '👁'}
              </button>
            </div>
          </div>

          <div class="form-group">
            <label>{t('entry.notes')}</label>
            <textarea
              value={notes}
              onInput={(e) => setNotes(e.currentTarget.value)}
              placeholder={t('entry.notes_placeholder')}
              rows={3}
            />
          </div>

          <div class="form-group">
            <label>
              {t('entry.totp_secret')}
              {entry?.has_totp ? ` (${t('entry.totp_keep_hint')})` : ''}
            </label>
            <input
              type="text"
              value={totpSecret}
              onInput={(e) => setTotpSecret(e.currentTarget.value)}
              placeholder={t('entry.totp_secret_placeholder')}
            />
          </div>

          <div class="form-group">
            <label>{t('entry.folder')}</label>
            <select value={folderId} onChange={(e) => setFolderId((e.target as HTMLSelectElement).value)}>
              <option value="">— None —</option>
              {folders.map((f) => (
                <option value={f.id} key={f.id}>{f.name}</option>
              ))}
            </select>
          </div>

          <div class="form-group">
            <label>{t('entry.tags')}</label>
            <div class="tag-selector">
              {allTags.map((tag) => (
                <button
                  key={tag.id}
                  class={`tag-chip ${selectedTagIds.includes(tag.id) ? 'selected' : ''}`}
                  onClick={() => toggleTag(tag.id)}
                >
                  {tag.name}
                </button>
              ))}
            </div>
          </div>

          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input
                type="checkbox"
                checked={isFavorite}
                onChange={() => setIsFavorite(!isFavorite)}
              />
              {t('entry.is_favorite')}
            </label>
          </div>
        </div>

        <div class="form-actions">
          <button class="btn btn-secondary" onClick={onClose}>
            {t('common.cancel')}
          </button>
          <button
            class="btn btn-primary"
            onClick={handleSave}
            disabled={saving || !title.trim()}
          >
            {saving ? t('common.loading') : t('common.save')}
          </button>
        </div>
      </div>
    </div>
  );
}
