import { useState, useEffect } from 'preact/hooks';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../stores/app';
import * as api from '../api';
import './TOTPDisplay.css';

interface TOTPDisplayProps {
  entryId: string;
}

export default function TOTPDisplay({ entryId }: TOTPDisplayProps) {
  const { t } = useTranslation();
  const { darkMode, addNotification } = useAppStore();
  const [code, setCode] = useState('');
  const [timeRemaining, setTimeRemaining] = useState(0);

  const generate = async () => {
    try {
      const result = await api.generateTOTPForEntry(entryId);
      setCode(result.code);
      setTimeRemaining(result.time_remaining);
    } catch {
      setCode('');
    }
  };

  useEffect(() => {
    if (!entryId) return;
    generate();

    const interval = setInterval(() => {
      const epoch = Math.floor(Date.now() / 1000);
      const remaining = 30 - (epoch % 30);
      setTimeRemaining(remaining);

      if (remaining === 30 || remaining === 0) {
        generate();
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [entryId]);

  const handleCopy = async () => {
    try {
      await api.copyTOTP(entryId);
      addNotification(t('notif.copied_clipboard') + ' — ' + t('notif.clipboard_warning'), 'info');
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Error';
      addNotification(msg, 'error');
    }
  };

  return (
    <div class="totp-display" data-theme={darkMode ? 'dark' : 'light'}>
      <div class="totp-header">
        <span class="totp-label">{t('totp.title')}</span>
        <span class="totp-timer">
          {t('totp.time_remaining', { seconds: timeRemaining })}
        </span>
      </div>

      <div class="totp-code" onClick={handleCopy}>
        {code ? `${code.slice(0, 3)} ${code.slice(3)}` : '------'}
      </div>

      <div class="totp-progress">
        <div
          class="totp-progress-bar"
          style={{ width: `${(timeRemaining / 30) * 100}%` }}
        />
      </div>
    </div>
  );
}
