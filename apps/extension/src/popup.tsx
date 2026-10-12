import { render } from 'preact';
import type { TargetedEvent } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import {
  Avatar,
  DiceIcon,
  EyeIcon,
  GearIcon,
  KeyIcon,
  LockIcon,
  Mark,
  SearchIcon,
  Switch,
  UserIcon,
  VaultIcon,
} from '@rvaultpass/ui';
import '@rvaultpass/ui/styles.css';
import type { ExtensionRequest, ExtensionResponse, PublicEntry } from './messages';
import { safeHttpOrigin } from './origin';
import './popup.css';
import './skin.css';

type PopupTab = 'vault' | 'generator' | 'settings';

interface Toast {
  tone: 'error' | 'ok';
  text: string;
}

function send(payload: ExtensionRequest): Promise<ExtensionResponse> {
  return chrome.runtime.sendMessage(payload);
}

function Popup() {
  const [ready, setReady] = useState(false);
  const [unlocked, setUnlocked] = useState(false);
  const [serverUrl, setServerUrl] = useState('http://127.0.0.1:8443');
  const [serverOpen, setServerOpen] = useState(false);
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState<PopupTab>('vault');
  const [query, setQuery] = useState('');
  const [entries, setEntries] = useState<PublicEntry[]>([]);
  const [siteHost, setSiteHost] = useState('');
  const [siteOrigin, setSiteOrigin] = useState('');
  const [tabId, setTabId] = useState<number | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);
  const [epoch, setEpoch] = useState(0);
  const [length, setLength] = useState(16);
  const [uppercase, setUppercase] = useState(true);
  const [lowercase, setLowercase] = useState(true);
  const [numbers, setNumbers] = useState(true);
  const [symbols, setSymbols] = useState(true);
  const [ambiguous, setAmbiguous] = useState(true);
  const [generated, setGenerated] = useState('');

  useEffect(() => {
    if (!toast) return undefined;
    const timer = setTimeout(() => setToast(null), 2200);
    return () => clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(async () => {
      const current = await send({ type: 'status' });
      if (cancelled) return;
      if (current.error) {
        setToast({ tone: 'error', text: current.error });
        setReady(true);
        return;
      }
      setUnlocked(Boolean(current.unlocked));
      if (current.serverUrl) setServerUrl(current.serverUrl);
      else setServerOpen(true);
      if (!current.unlocked) {
        setEntries([]);
        setReady(true);
        return;
      }
      const page = await activePage();
      if (cancelled) return;
      setTabId(page.tabId);
      setSiteHost(page.host);
      setSiteOrigin(page.origin);
      const result = query.trim()
        ? await send({ type: 'search', query: query.trim() })
        : await send({ type: 'matches', origin: page.origin });
      if (cancelled) return;
      if (result.error) setToast({ tone: 'error', text: result.error });
      setEntries(result.entries || []);
      setReady(true);
    }, 160);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query, epoch]);

  useEffect(() => {
    if (!unlocked || tab !== 'generator') return undefined;
    let cancelled = false;
    const timer = setTimeout(async () => {
      const response = await send({
        type: 'generate',
        params: {
          length,
          use_uppercase: uppercase,
          use_lowercase: lowercase,
          use_numbers: numbers,
          use_symbols: symbols,
          exclude_ambiguous: ambiguous,
        },
      });
      if (cancelled) return;
      if (response.error) {
        setToast({ tone: 'error', text: response.error });
        return;
      }
      setGenerated(response.password || '');
    }, 200);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [unlocked, tab, length, uppercase, lowercase, numbers, symbols, ambiguous]);

  const unlock = async (event: TargetedEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!password || busy) return;
    setBusy(true);
    setToast(null);
    try {
      const saved = await send({ type: 'save-server', serverUrl: serverUrl.trim() });
      if (saved.error) throw new Error(saved.error);
      const response = await send({ type: 'unlock', password });
      if (response.error) throw new Error(response.error);
      setPassword('');
      setUnlocked(true);
      setEpoch((value) => value + 1);
    } catch (error) {
      setToast({ tone: 'error', text: error instanceof Error ? error.message : 'Erreur inattendue' });
    } finally {
      setBusy(false);
    }
  };

  const lock = async () => {
    await send({ type: 'lock' });
    setUnlocked(false);
    setEntries([]);
    setGenerated('');
    setQuery('');
    setTab('vault');
    setToast({ tone: 'ok', text: 'Coffre verrouillé' });
    setEpoch((value) => value + 1);
  };

  const fill = async (entry: PublicEntry) => {
    if (entry.origin !== siteOrigin || tabId == null) {
      setToast({ tone: 'error', text: 'Cette entrée ne correspond pas à la page ouverte' });
      return;
    }
    const response = await send({ type: 'fill', entryId: entry.id, tabId });
    if (response.error) {
      setToast({ tone: 'error', text: response.error });
      return;
    }
    window.close();
  };

  const copyText = async (value: string, label: string) => {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(value);
      setToast({ tone: 'ok', text: label });
    } catch (error) {
      setToast({ tone: 'error', text: error instanceof Error ? error.message : 'Erreur inattendue' });
    }
  };

  const copyPassword = async (entry: PublicEntry) => {
    const response = await send({ type: 'credential', entryId: entry.id });
    if (response.error) {
      setToast({ tone: 'error', text: response.error });
      return;
    }
    await copyText(response.password || '', 'Mot de passe copié');
  };

  const saveServer = async (event: TargetedEvent<HTMLFormElement>) => {
    event.preventDefault();
    const response = await send({ type: 'save-server', serverUrl: serverUrl.trim() });
    setToast(response.error
      ? { tone: 'error', text: response.error }
      : { tone: 'ok', text: 'Adresse enregistrée' });
  };

  const keepClass = (candidate: boolean, snapshot: Record<string, boolean>) => {
    const enabled = Object.values(snapshot).filter(Boolean).length;
    return enabled === 0 ? !candidate : candidate;
  };

  if (!ready) {
    return <div class="dl-app"><p class="dl-loading">Ouverture…</p></div>;
  }

  return (
    <div class="dl-app">
      {unlocked ? (
        <Vault
          toast={toast}
          query={query}
          onQuery={setQuery}
          tab={tab}
          onTab={setTab}
          entries={entries}
          siteHost={siteHost}
          onFill={fill}
          onCopyUser={(entry) => copyText(entry.username, 'Identifiant copié')}
          onCopyPassword={copyPassword}
          onLock={lock}
          length={length}
          onLength={setLength}
          uppercase={uppercase}
          lowercase={lowercase}
          numbers={numbers}
          symbols={symbols}
          ambiguous={ambiguous}
          onUppercase={(value) => setUppercase(keepClass(value, { uppercase: value, lowercase, numbers, symbols }))}
          onLowercase={(value) => setLowercase(keepClass(value, { uppercase, lowercase: value, numbers, symbols }))}
          onNumbers={(value) => setNumbers(keepClass(value, { uppercase, lowercase, numbers: value, symbols }))}
          onSymbols={(value) => setSymbols(keepClass(value, { uppercase, lowercase, numbers, symbols: value }))}
          onAmbiguous={setAmbiguous}
          generated={generated}
          onCopyGenerated={() => copyText(generated, 'Mot de passe copié')}
          serverUrl={serverUrl}
          onServerUrl={setServerUrl}
          onSaveServer={saveServer}
        />
      ) : (
        <form class="dl-lock" onSubmit={unlock}>
          <div class="dl-mark-wrap"><Mark size={56} /></div>
          <h1>RVaultPass</h1>
          <p class="dl-lead">Saisissez votre mot de passe maître</p>
          <label class="dl-field">
            <span>Mot de passe maître</span>
            <span class="dl-password">
              <input
                type={showPassword ? 'text' : 'password'}
                value={password}
                autocomplete="current-password"
                onInput={(event: TargetedEvent<HTMLInputElement>) => setPassword(event.currentTarget.value)}
              />
              <button
                type="button"
                class="dl-eye"
                aria-label={showPassword ? 'Masquer le mot de passe' : 'Afficher le mot de passe'}
                onClick={() => setShowPassword((value) => !value)}
              >
                <EyeIcon off={showPassword} />
              </button>
            </span>
          </label>
          {serverOpen ? (
            <label class="dl-field">
              <span>Adresse du service</span>
              <input
                type="url"
                value={serverUrl}
                autocomplete="url"
                onInput={(event: TargetedEvent<HTMLInputElement>) => setServerUrl(event.currentTarget.value)}
              />
            </label>
          ) : null}
          <button class="dl-primary" type="submit" disabled={busy || !password}>
            {busy ? 'Déverrouillage…' : 'Déverrouiller'}
          </button>
          <button class="dl-link" type="button" onClick={() => setServerOpen((value) => !value)}>
            {serverOpen ? 'Masquer le service' : 'Adresse du service'}
          </button>
        </form>
      )}
      {!unlocked && toast ? <p class={toast.tone === 'error' ? 'dl-toast is-error' : 'dl-toast'} role="status">{toast.text}</p> : null}
    </div>
  );
}

interface VaultProps {
  toast: Toast | null;
  query: string;
  onQuery: (value: string) => void;
  tab: PopupTab;
  onTab: (tab: PopupTab) => void;
  entries: PublicEntry[];
  siteHost: string;
  onFill: (entry: PublicEntry) => void;
  onCopyUser: (entry: PublicEntry) => void;
  onCopyPassword: (entry: PublicEntry) => void;
  onLock: () => void;
  length: number;
  onLength: (value: number) => void;
  uppercase: boolean;
  lowercase: boolean;
  numbers: boolean;
  symbols: boolean;
  ambiguous: boolean;
  onUppercase: (value: boolean) => void;
  onLowercase: (value: boolean) => void;
  onNumbers: (value: boolean) => void;
  onSymbols: (value: boolean) => void;
  onAmbiguous: (value: boolean) => void;
  generated: string;
  onCopyGenerated: () => void;
  serverUrl: string;
  onServerUrl: (value: string) => void;
  onSaveServer: (event: TargetedEvent<HTMLFormElement>) => void;
}

function Vault({
  query, onQuery, tab, onTab, entries, siteHost, onFill, onCopyUser, onCopyPassword, onLock,
  length, onLength, uppercase, lowercase, numbers, symbols, ambiguous,
  onUppercase, onLowercase, onNumbers, onSymbols, onAmbiguous,
  generated, onCopyGenerated, serverUrl, onServerUrl, onSaveServer, toast,
}: VaultProps) {
  return (
    <>
      <header class="dl-header">
        <div class="dl-brand"><Mark size={28} />RVaultPass</div>
        <button type="button" class="dl-icon" aria-label="Verrouiller" onClick={onLock}><LockIcon size={18} /></button>
      </header>
      {tab === 'vault' ? (
        <>
          <label class="dl-search">
            <SearchIcon />
            <input
              type="search"
              value={query}
              placeholder="Rechercher un identifiant"
              onInput={(event: TargetedEvent<HTMLInputElement>) => onQuery(event.currentTarget.value)}
            />
          </label>
          <div class="dl-scroll">
            <p class="dl-section">{query.trim() ? 'Résultats' : siteHost ? `Pour ${siteHost}` : 'Pour ce site'}</p>
            {entries.length === 0 ? (
              <p class="dl-empty">
                {query.trim() ? 'Aucun identifiant ne correspond.' : 'Aucun identifiant pour ce site. Utilisez la recherche.'}
              </p>
            ) : entries.map((entry) => (
              <article class="dl-row" key={entry.id}>
                <button type="button" class="dl-row-main" onClick={() => onFill(entry)}>
                  <Avatar title={entry.title} />
                  <span class="dl-row-text">
                    <strong>{entry.title || 'Sans titre'}</strong>
                    <small>{entry.username || hostOf(entry.url) || 'Sans identifiant'}</small>
                  </span>
                </button>
                <button type="button" class="dl-icon" aria-label="Copier l'identifiant" disabled={!entry.username} onClick={() => onCopyUser(entry)}>
                  <UserIcon />
                </button>
                <button type="button" class="dl-icon" aria-label="Copier le mot de passe" onClick={() => onCopyPassword(entry)}>
                  <KeyIcon />
                </button>
              </article>
            ))}
          </div>
        </>
      ) : null}
      {tab === 'generator' ? (
        <div class="dl-scroll">
          <section class="dl-generator">
            <h2>Générateur</h2>
            <div class="dl-secret">
              <code>{generated || '…'}</code>
              <button type="button" class="dl-icon" aria-label="Copier le mot de passe" disabled={!generated} onClick={onCopyGenerated}>
                <KeyIcon />
              </button>
            </div>
            <div class="dl-length"><span>Longueur</span><strong>{length}</strong></div>
            <input type="range" min="8" max="40" value={length} onInput={(event: TargetedEvent<HTMLInputElement>) => onLength(Number(event.currentTarget.value))} />
            <Switch label="Majuscules" checked={uppercase} onChange={onUppercase} />
            <Switch label="Minuscules" checked={lowercase} onChange={onLowercase} />
            <Switch label="Chiffres" checked={numbers} onChange={onNumbers} />
            <Switch label="Symboles" checked={symbols} onChange={onSymbols} />
            <Switch label="Éviter les caractères ambigus" checked={ambiguous} onChange={onAmbiguous} />
          </section>
        </div>
      ) : null}
      {tab === 'settings' ? (
        <div class="dl-scroll">
          <form class="dl-settings" onSubmit={onSaveServer}>
            <h2>Réglages</h2>
            <p class="dl-lead">Le coffre reste sur le service. Cette page ne conserve pas le mot de passe maître.</p>
            <label class="dl-field">
              <span>Adresse du service</span>
              <input type="url" value={serverUrl} autocomplete="url" onInput={(event: TargetedEvent<HTMLInputElement>) => onServerUrl(event.currentTarget.value)} />
            </label>
            <button class="dl-primary" type="submit">Enregistrer</button>
            <button class="dl-ghost" type="button" onClick={onLock}>Verrouiller le coffre</button>
          </form>
        </div>
      ) : null}
      {toast ? <p class={toast.tone === 'error' ? 'dl-toast is-error' : 'dl-toast'} role="status">{toast.text}</p> : null}
      <nav class="dl-nav">
        <button type="button" class={tab === 'vault' ? 'is-on' : ''} onClick={() => onTab('vault')}><VaultIcon />Coffre</button>
        <button type="button" class={tab === 'generator' ? 'is-on' : ''} onClick={() => onTab('generator')}><DiceIcon />Générateur</button>
        <button type="button" class={tab === 'settings' ? 'is-on' : ''} onClick={() => onTab('settings')}><GearIcon />Réglages</button>
      </nav>
    </>
  );
}

async function activePage() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return {
    tabId: tab?.id ?? null,
    origin: originOf(tab?.url),
    host: hostOf(tab?.url),
  };
}

function originOf(url?: string) {
  return safeHttpOrigin(url) ?? '';
}

function hostOf(url?: string) {
  if (!url) return '';
  try { return new URL(url).hostname.replace(/^www\./, ''); } catch { return ''; }
}

const root = document.querySelector('#app');
if (root) render(<Popup />, root);
