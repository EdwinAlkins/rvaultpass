import { render } from 'preact';
import { useEffect, useMemo, useState } from 'preact/hooks';
import { createClient } from '@rvaultpass/vault-client';
import type { EntrySummary, VaultClient, VaultFolder, VaultTag } from '@rvaultpass/vault-client';
import {
  Brand,
  Button,
  Collection,
  DiceIcon,
  EntryComposer,
  EntryDetail,
  EntryList,
  FolderIcon,
  GeneratorPanel,
  IconButton,
  LockIcon,
  NavItem,
  Notice,
  SearchField,
  TagIcon,
  Toolbar,
  UnlockScreen,
  VaultIcon,
  Workspace,
} from '@rvaultpass/ui';
import type { EntryDraft, NoticeTone } from '@rvaultpass/ui';
import '@rvaultpass/ui/styles.css';

const DEFAULT_SERVER = 'http://127.0.0.1:8443';

interface ViewFilter {
  kind: 'all' | 'folder' | 'tag';
  id: string | null;
}

function WebApp() {
  const [serverUrl, setServerUrl] = useState(localStorage.getItem('rvault.serverUrl') || DEFAULT_SERVER);
  const [token, setToken] = useState<string | null>(null);
  const [entries, setEntries] = useState<EntrySummary[]>([]);
  const [folders, setFolders] = useState<VaultFolder[]>([]);
  const [tags, setTags] = useState<VaultTag[]>([]);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<ViewFilter>({ kind: 'all', id: null });
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);
  const [secret, setSecret] = useState<string | null>(null);
  const [code, setCode] = useState<string | null>(null);
  const [generated, setGenerated] = useState('');
  const [showGenerator, setShowGenerator] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [tone, setTone] = useState<NoticeTone>('error');

  const api = (): VaultClient => {
    const client = createClient(serverUrl);
    if (token) client.setToken(token);
    return client;
  };

  const fail = (error: unknown) => {
    setTone('error');
    setMessage(error instanceof Error ? error.message : 'Erreur inattendue');
  };

  const reload = async (client: VaultClient = api()) => {
    const [nextEntries, nextFolders, nextTags] = await Promise.all([
      client.entries(),
      client.folders(),
      client.tags(),
    ]);
    setEntries(nextEntries);
    setFolders(nextFolders);
    setTags(nextTags);
  };

  useEffect(() => {
    setSecret(null);
    setCode(null);
  }, [selectedId]);

  useEffect(() => {
    if (!token) return undefined;
    let cancelled = false;
    const timer = setTimeout(async () => {
      try {
        const client = createClient(serverUrl);
        client.setToken(token);
        const next = query.trim() ? await client.search(query.trim()) : await client.entries();
        if (!cancelled) setEntries(next);
      } catch (error) {
        if (!cancelled) fail(error);
      }
    }, 180);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query, token, serverUrl]);

  const visible = useMemo(() => {
    if (query.trim()) return entries;
    if (filter.kind === 'folder') return entries.filter((entry) => entry.folder_id === filter.id);
    if (filter.kind === 'tag') return entries.filter((entry) => entry.tags.some((tag) => tag.id === filter.id));
    return entries;
  }, [entries, filter, query]);

  const selected = entries.find((entry) => entry.id === selectedId) || null;
  const title = query.trim()
    ? 'Recherche'
    : filter.kind === 'folder'
      ? folders.find((folder) => folder.id === filter.id)?.name || 'Dossier'
      : filter.kind === 'tag'
        ? tags.find((tag) => tag.id === filter.id)?.name || 'Étiquette'
        : 'Identifiants';

  const unlock = async (password: string) => {
    setBusy(true);
    setMessage('');
    try {
      localStorage.setItem('rvault.serverUrl', serverUrl);
      const client = createClient(serverUrl);
      const session = await client.unlock(password);
      setToken(session.token);
      await reload(client);
      setTone('ok');
      setMessage('');
    } catch (error) {
      fail(error);
      throw error;
    } finally {
      setBusy(false);
    }
  };

  const lock = async () => {
    try {
      if (token) await api().lock();
    } catch {
      // The session may already be gone.
    }
    setToken(null);
    setEntries([]);
    setFolders([]);
    setTags([]);
    setSelectedId(null);
    setComposing(false);
    setSecret(null);
    setCode(null);
    setGenerated('');
    setTone('info');
    setMessage('Coffre verrouillé');
  };

  const createEntry = async (body: EntryDraft) => {
    setBusy(true);
    try {
      const created = await api().createEntry(body);
      setQuery('');
      await reload();
      setSelectedId(created.id);
      setComposing(false);
      setTone('ok');
      setMessage('Identifiant enregistré');
    } catch (error) {
      fail(error);
      throw error;
    } finally {
      setBusy(false);
    }
  };

  const revealPassword = async () => {
    if (!selected) return;
    try {
      const result = await api().password(selected.id);
      setSecret(result.password);
    } catch (error) {
      fail(error);
    }
  };

  const revealTotp = async () => {
    if (!selected) return;
    try {
      const result = await api().totp(selected.id);
      setCode(result.code);
    } catch (error) {
      fail(error);
    }
  };

  const removeEntry = async () => {
    if (!selected) return;
    if (!window.confirm(`Supprimer « ${selected.title || 'Sans titre'} » ?`)) return;
    try {
      await api().deleteEntry(selected.id);
      setSelectedId(null);
      await reload();
      setTone('ok');
      setMessage('Identifiant supprimé');
    } catch (error) {
      fail(error);
    }
  };

  const addNamed = async (create: () => Promise<unknown>) => {
    try {
      await create();
      await reload();
    } catch (error) {
      fail(error);
      throw error;
    }
  };

  const copy = async (value: string) => {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(value);
      setTone('ok');
      setMessage('Copié dans le presse-papiers');
    } catch (error) {
      fail(error);
    }
  };

  const generate = async () => {
    try {
      const result = await api().generatePassword({
        length: 20,
        use_uppercase: true,
        use_lowercase: true,
        use_numbers: true,
        use_symbols: true,
        exclude_ambiguous: true,
      });
      setGenerated(result.password);
      setShowGenerator(true);
    } catch (error) {
      fail(error);
    }
  };

  if (!token) {
    return (
      <UnlockScreen
        serverUrl={serverUrl}
        onServerUrl={setServerUrl}
        busy={busy}
        message={message}
        tone={tone}
        onUnlock={unlock}
      />
    );
  }

  return (
    <Workspace
      sidebar={(
        <>
          <Brand />
          <NavItem
            icon={<VaultIcon />}
            label="Identifiants"
            active={filter.kind === 'all' && !query.trim()}
            onClick={() => { setFilter({ kind: 'all', id: null }); setQuery(''); }}
          />
          <Collection
            title="Dossiers"
            icon={<FolderIcon />}
            items={folders}
            selectedId={filter.kind === 'folder' ? filter.id : null}
            empty="Aucun dossier."
            onSelect={(folder) => { setFilter({ kind: 'folder', id: folder.id }); setQuery(''); setSelectedId(null); }}
            onAdd={(name) => addNamed(() => api().createFolder(name))}
          />
          <Collection
            title="Étiquettes"
            icon={<TagIcon />}
            items={tags}
            selectedId={filter.kind === 'tag' ? filter.id : null}
            empty="Aucune étiquette."
            onSelect={(tag) => { setFilter({ kind: 'tag', id: tag.id }); setQuery(''); setSelectedId(null); }}
            onAdd={(name) => addNamed(() => api().createTag(name))}
          />
        </>
      )}
      toolbar={(
        <>
          <Toolbar title={title} search={<SearchField value={query} onInput={setQuery} />}>
            <Button size="sm" onClick={() => { setComposing(true); setSelectedId(null); }}>Ajouter</Button>
            <IconButton label="Générateur" onClick={() => setShowGenerator((value) => !value)}><DiceIcon /></IconButton>
            <IconButton label="Verrouiller" onClick={lock}><LockIcon size={18} /></IconButton>
          </Toolbar>
          {showGenerator ? (
            <div class="rv-pad">
              <GeneratorPanel value={generated} onGenerate={generate} onCopy={() => copy(generated)} />
            </div>
          ) : null}
          <Notice tone={tone}>{message}</Notice>
        </>
      )}
      list={(
        <EntryList
          entries={visible}
          selectedId={selectedId}
          empty={query.trim() ? 'Aucun identifiant ne correspond.' : 'Aucun identifiant dans cette vue.'}
          onSelect={(entry) => { setSelectedId(entry.id); setComposing(false); }}
        />
      )}
      detail={composing ? (
        <EntryComposer busy={busy} onSubmit={createEntry} onCancel={() => setComposing(false)} />
      ) : (
        <EntryDetail
          entry={selected}
          secret={secret}
          code={code}
          onRevealPassword={revealPassword}
          onRevealTotp={revealTotp}
          onDelete={removeEntry}
          onCopy={copy}
        />
      )}
    />
  );
}

const root = document.querySelector('#app');
if (root) render(<WebApp />, root);
