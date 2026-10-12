import { render } from 'preact';
import type { TargetedEvent } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import { Mark } from '@rvaultpass/ui';
import '@rvaultpass/ui/styles.css';
import type { ExtensionResponse } from './messages';
import './skin.css';

function Options() {
  const [serverUrl, setServerUrl] = useState('http://127.0.0.1:8443');
  const [message, setMessage] = useState('');
  const [tone, setTone] = useState<'ok' | 'error'>('ok');

  useEffect(() => {
    chrome.storage.local.get('serverUrl').then((stored) => {
      if (typeof stored.serverUrl === 'string' && stored.serverUrl) setServerUrl(stored.serverUrl);
    });
  }, []);

  const save = async (event: TargetedEvent<HTMLFormElement>) => {
    event.preventDefault();
    const response = await chrome.runtime.sendMessage({
      type: 'save-server',
      serverUrl: serverUrl.trim(),
    }) as ExtensionResponse;
    setTone(response.error ? 'error' : 'ok');
    setMessage(response.error || 'Adresse enregistrée');
  };

  return (
    <div class="dl-options">
      <form class="dl-card" onSubmit={save}>
        <div class="dl-mark-wrap"><Mark size={56} /></div>
        <h1>RVaultPass</h1>
        <p class="dl-lead">Le popup utilise cette adresse pour joindre le service.</p>
        <label class="dl-field">
          <span>Adresse du service</span>
          <input
            type="url"
            value={serverUrl}
            autocomplete="url"
            onInput={(event: TargetedEvent<HTMLInputElement>) => setServerUrl(event.currentTarget.value)}
          />
        </label>
        {message ? <p class={tone === 'error' ? 'dl-toast is-error' : 'dl-toast'} role="status">{message}</p> : null}
        <button class="dl-primary" type="submit">Enregistrer</button>
      </form>
    </div>
  );
}

const root = document.querySelector('#app');
if (root) render(<Options />, root);
