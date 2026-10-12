import { createClient } from '@rvaultpass/vault-client';
import type { EntrySummary, PasswordGenerationParams } from '@rvaultpass/vault-client';
import { isExtensionRequest } from './messages';
import type { ExtensionRequest, FillRequest, PublicEntry } from './messages';
import { originsMatch, safeHttpOrigin, sameDocument } from './origin';


chrome.runtime.onMessage.addListener((message: unknown, _sender, sendResponse) => {
  if (!isExtensionRequest(message)) {
    sendResponse({ error: 'Unknown message' });
    return false;
  }
  handle(message).then(sendResponse).catch((error: unknown) => {
    sendResponse({ error: error instanceof Error ? error.message : 'Erreur inattendue' });
  });
  return true;
});

async function handle(message: ExtensionRequest) {
  if (message.type === 'status') return status();
  if (message.type === 'save-server') return saveServer(message.serverUrl);
  if (message.type === 'unlock') return unlock(message.password);
  if (message.type === 'lock') return lock();
  if (message.type === 'search') return { entries: await search(message.query) };
  if (message.type === 'matches') return { entries: await matches(message.origin) };
  if (message.type === 'fill') return fill(message.entryId, message.tabId);
  if (message.type === 'credential') return credential(message.entryId);
  if (message.type === 'generate') return generate(message.params);
  const unexpected: never = message;
  throw new Error(`Unknown message ${unexpected}`);
}

async function status() {
  const config = await chrome.storage.local.get(['serverUrl']);
  const serverUrl = typeof config.serverUrl === 'string' ? config.serverUrl : '';
  const token = await sessionToken();
  if (!token) return { unlocked: false, serverUrl };
  try {
    const client = await clientFor();
    const remote = await client.status();
    if (!remote.open) await chrome.storage.session.remove('token');
    return { unlocked: remote.open, serverUrl };
  } catch {
    await chrome.storage.session.remove('token');
    return { unlocked: false, serverUrl };
  }
}

async function saveServer(serverUrl: string) {
  await chrome.storage.local.set({ serverUrl });
  return { ok: true };
}

async function unlock(password: string) {
  const client = await clientFor();
  const session = await client.unlock(password);
  await chrome.storage.session.set({ token: session.token });
  const entries = await client.entries();
  return { count: entries.length };
}

async function lock() {
  try {
    const client = await clientFor();
    await client.lock();
  } catch {
    // Already locked on the server.
  }
  await chrome.storage.session.remove('token');
  return { ok: true };
}

async function search(query: string) {
  const client = await clientFor();
  const entries = query ? await client.search(query) : await client.entries();
  return publicEntries(entries);
}

async function matches(origin: string) {
  const client = await clientFor();
  const entries = await client.entries();
  return publicEntries(entries).filter((entry) => originsMatch(origin, entry.origin));
}

async function fill(entryId: string, tabId: number) {
  const client = await clientFor();
  const entry = await client.entry(entryId);
  const entryOrigin = safeHttpOrigin(entry.url);
  const before = await chrome.tabs.get(tabId);
  if (!entryOrigin || !sameDocument(before, before) || safeHttpOrigin(before.url) !== entryOrigin) {
    throw new Error("L'origine de la page ne correspond pas à l'entrée");
  }
  const secret = await client.password(entryId);
  const after = await chrome.tabs.get(tabId);
  if (!sameDocument(before, after)) {
    throw new Error('La page a changé avant le remplissage');
  }
  const payload: FillRequest = {
    type: 'fill',
    username: entry.username || '',
    password: secret.password || '',
    origin: entryOrigin,
  };
  const filled = await deliver(tabId, payload, (after as { documentId?: string }).documentId);
  if (!filled?.ok) throw new Error('Aucun formulaire de connexion sur cette page');
  return { ok: true };
}

async function credential(entryId: string) {
  const client = await clientFor();
  const secret = await client.password(entryId);
  return { password: secret.password || '' };
}

async function generate(params: Partial<PasswordGenerationParams> = {}) {
  const client = await clientFor();
  const result = await client.generatePassword({
    length: Number(params.length) || 16,
    use_uppercase: params.use_uppercase !== false,
    use_lowercase: params.use_lowercase !== false,
    use_numbers: params.use_numbers !== false,
    use_symbols: params.use_symbols !== false,
    exclude_ambiguous: params.exclude_ambiguous !== false,
  });
  return { password: result.password };
}

async function deliver(tabId: number, payload: FillRequest, documentId?: string) {
  const target: chrome.scripting.InjectionTarget = documentId
    ? { tabId, documentIds: [documentId] }
    : { tabId };
  try {
    return await chrome.tabs.sendMessage(tabId, payload) as { ok?: boolean } | undefined;
  } catch {
    await chrome.scripting.executeScript({ target, files: ['content.js'] });
    return chrome.tabs.sendMessage(tabId, payload) as Promise<{ ok?: boolean } | undefined>;
  }
}

function publicEntries(entries: EntrySummary[]): PublicEntry[] {
  return entries.map((entry) => ({
    id: entry.id,
    title: entry.title,
    username: entry.username || '',
    url: entry.url || '',
    origin: safeHttpOrigin(entry.url) || '',
  }));
}

async function clientFor() {
  const stored = await chrome.storage.local.get(['serverUrl']);
  if (typeof stored.serverUrl !== 'string' || !stored.serverUrl) {
    throw new Error('Indiquez l’URL du service dans le popup');
  }
  const client = createClient(stored.serverUrl);
  const token = await sessionToken();
  if (token) client.setToken(token);
  return client;
}

async function sessionToken() {
  try {
    const stored = await chrome.storage.session.get('token');
    return typeof stored.token === 'string' && stored.token ? stored.token : null;
  } catch {
    return null;
  }
}
