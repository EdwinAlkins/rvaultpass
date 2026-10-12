/**
 * Client HTTP du service de coffre.
 *
 * Le navigateur ne reçoit jamais le fichier `.vault`.
 * Le mot de passe maître ne sert qu'à {@link VaultClient.unlock} et n'est pas conservé.
 * Les listes ne contiennent pas les secrets : ils passent par {@link VaultClient.password}
 * et {@link VaultClient.totp}.
 */

/** Session ouverte après un déverrouillage réussi. */
export interface VaultSession {
  /** Jeton présenté dans l'en-tête `Authorization`. Ce n'est pas le mot de passe maître. */
  token: string;
  /** Expiration du jeton, en secondes depuis l'époque Unix. */
  expires_at: number;
}

/** État du coffre pour le jeton courant. */
export interface VaultStatus {
  /** Vrai tant qu'une session est ouverte sur le service. */
  open: boolean;
  /** Expiration du jeton, en secondes depuis l'époque Unix. */
  expires_at: number;
}

/** Étiquette associée à un identifiant. Ne contient aucun secret. */
export interface VaultTag {
  id: string;
  name: string;
}

/**
 * Résumé d'un identifiant.
 *
 * Les champs `has_*` indiquent qu'un secret existe, sans le transmettre.
 */
export interface EntrySummary {
  id: string;
  folder_id: string | null;
  title: string;
  url: string | null;
  username: string | null;
  username2: string | null;
  username3: string | null;
  created_at: string;
  updated_at: string;
  is_favorite: boolean;
  tags: VaultTag[];
  has_password: boolean;
  has_totp: boolean;
  has_notes: boolean;
}

/**
 * Métadonnées modifiables d'un identifiant.
 *
 * Le mot de passe et le secret TOTP n'en font pas partie.
 */
export interface EntryEditData {
  id: string;
  folder_id: string | null;
  title: string;
  url: string | null;
  username: string | null;
  username2: string | null;
  username3: string | null;
  notes: string | null;
  is_favorite: boolean;
  tags: VaultTag[];
  has_password: boolean;
  has_totp: boolean;
}

/** Données acceptées pour créer un identifiant. `password` peut être une chaîne vide. */
export interface NewEntry {
  title: string;
  password: string;
  folder_id?: string | null;
  url?: string | null;
  username?: string | null;
  username2?: string | null;
  username3?: string | null;
  notes?: string | null;
  totp_secret?: string | null;
  is_favorite?: boolean;
  tag_ids?: string[];
}

/** Champs envoyés pour modifier un identifiant. Les champs absents restent inchangés. */
export interface EntryUpdate {
  folder_id?: string | null;
  title?: string;
  url?: string | null;
  username?: string | null;
  username2?: string | null;
  username3?: string | null;
  password?: string;
  notes?: string | null;
  totp_secret?: string | null;
  is_favorite?: boolean;
  tag_ids?: string[];
}

/** Dossier du coffre. */
export interface VaultFolder {
  id: string;
  name: string;
  parent_id: string | null;
  created_at: string;
  updated_at: string;
}

/** Mot de passe révélé pour un identifiant. */
export interface RevealedPassword {
  password: string;
}

/** Code TOTP courant et secondes restantes avant le prochain. */
export interface TotpCode {
  code: string;
  time_remaining: number;
}

/**
 * Contraintes du générateur de mots de passe.
 *
 * Les noms suivent le corps JSON du service, en snake_case.
 * Une longueur hors de 4 à 1024 est refusée par le service.
 */
export interface PasswordGenerationParams {
  length: number;
  use_uppercase: boolean;
  use_lowercase: boolean;
  use_numbers: boolean;
  use_symbols: boolean;
  exclude_ambiguous: boolean;
}

/** Mot de passe produit par le service. Il n'est pas enregistré dans le coffre. */
export interface GeneratedPassword {
  password: string;
}

/**
 * Accès au service pour une adresse de base.
 *
 * Le jeton reste en mémoire dans le client. Chaque appel authentifié l'envoie,
 * puis le service l'oublie à l'expiration ou au verrouillage.
 */
export interface VaultClient {
  /** Jeton courant, ou `null` tant que le coffre n'est pas déverrouillé. */
  token(): string | null;
  /** Remplace le jeton, par exemple après une restauration de session. */
  setToken(value: string | null): void;
  /**
   * Ouvre une session.
   *
   * @param password Mot de passe maître. Il n'est ni stocké ni renvoyé.
   */
  unlock(password: string): Promise<VaultSession>;
  /** Ferme la session côté service et invalide le jeton. */
  lock(): Promise<void>;
  /** Indique si le coffre est encore ouvert pour ce jeton. */
  status(): Promise<VaultStatus>;
  /** Liste les résumés, sans mots de passe ni secrets TOTP. */
  entries(): Promise<EntrySummary[]>;
  /** Lit les métadonnées d'un identifiant, toujours sans ses secrets. */
  entry(id: string): Promise<EntryEditData>;
  /** Crée un identifiant et renvoie son résumé. */
  createEntry(body: NewEntry): Promise<EntrySummary>;
  /** Met à jour un identifiant et renvoie son résumé. */
  updateEntry(id: string, body: EntryUpdate): Promise<EntrySummary>;
  /** Supprime un identifiant. */
  deleteEntry(id: string): Promise<void>;
  /** Révèle le mot de passe d'un seul identifiant. */
  password(id: string): Promise<RevealedPassword>;
  /** Calcule le code TOTP courant d'un seul identifiant. */
  totp(id: string): Promise<TotpCode>;
  /** Liste les dossiers. */
  folders(): Promise<VaultFolder[]>;
  /**
   * Crée un dossier.
   *
   * @param parentId Dossier parent, ou `null` pour le placer à la racine.
   */
  createFolder(name: string, parentId?: string | null): Promise<VaultFolder>;
  /** Supprime un dossier. */
  deleteFolder(id: string): Promise<void>;
  /** Liste les étiquettes. */
  tags(): Promise<VaultTag[]>;
  /** Crée une étiquette. */
  createTag(name: string): Promise<VaultTag>;
  /** Supprime une étiquette. */
  deleteTag(id: string): Promise<void>;
  /** Recherche des résumés dont le titre, l'identifiant ou l'adresse contient `query`. */
  search(query: string): Promise<EntrySummary[]>;
  /** Demande un mot de passe au service. Le résultat n'est pas enregistré. */
  generatePassword(params: PasswordGenerationParams): Promise<GeneratedPassword>;
}

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE';
  token?: string | null;
  body?: unknown;
  query?: Record<string, string | number | boolean | null | undefined>;
}

/**
 * Crée un client pour un service déjà lancé.
 *
 * @param baseUrl Origine du service, par exemple `http://127.0.0.1:8443`.
 */
export function createClient(baseUrl: string): VaultClient {
  let token: string | null = null;
  const call = <T>(path: string, options: RequestOptions = {}) =>
    request<T>(baseUrl, path, { ...options, token });

  return {
    token: () => token,
    setToken(value) {
      token = value;
    },
    async unlock(password) {
      const session = await request<VaultSession>(baseUrl, '/v1/vault/unlock', {
        method: 'POST',
        body: { password },
      });
      token = session.token;
      return session;
    },
    lock: () => call<void>('/v1/vault/lock', { method: 'POST' }),
    status: () => call<VaultStatus>('/v1/vault/status'),
    entries: () => call<EntrySummary[]>('/v1/entries'),
    entry: (id) => call<EntryEditData>(`/v1/entries/${id}`),
    createEntry: (body) => call<EntrySummary>('/v1/entries', { method: 'POST', body }),
    updateEntry: (id, body) => call<EntrySummary>(`/v1/entries/${id}`, { method: 'PUT', body }),
    deleteEntry: (id) => call<void>(`/v1/entries/${id}`, { method: 'DELETE' }),
    password: (id) => call<RevealedPassword>(`/v1/entries/${id}/password`),
    totp: (id) => call<TotpCode>(`/v1/entries/${id}/totp`),
    folders: () => call<VaultFolder[]>('/v1/folders'),
    createFolder: (name, parentId = null) =>
      call<VaultFolder>('/v1/folders', { method: 'POST', body: { name, parent_id: parentId } }),
    deleteFolder: (id) => call<void>(`/v1/folders/${id}`, { method: 'DELETE' }),
    tags: () => call<VaultTag[]>('/v1/tags'),
    createTag: (name) => call<VaultTag>('/v1/tags', { method: 'POST', body: { name } }),
    deleteTag: (id) => call<void>(`/v1/tags/${id}`, { method: 'DELETE' }),
    search: (query) => call<EntrySummary[]>('/v1/search', { query: { q: query } }),
    generatePassword: (params) =>
      call<GeneratedPassword>('/v1/passwords/generate', { method: 'POST', body: params }),
  };
}

async function request<T>(baseUrl: string, path: string, options: RequestOptions = {}): Promise<T> {
  const url = new URL(path, baseUrl.endsWith('/') ? baseUrl : `${baseUrl}/`);
  if (options.query) {
    for (const [key, value] of Object.entries(options.query)) {
      if (value !== undefined && value !== null) url.searchParams.set(key, String(value));
    }
  }
  const headers: Record<string, string> = {};
  if (options.body !== undefined) headers['content-type'] = 'application/json';
  if (options.token) headers.authorization = `Bearer ${options.token}`;
  const response = await fetch(url, {
    method: options.method ?? 'GET',
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
  });
  if (response.status === 204) return undefined as T;
  const payload: unknown = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(errorMessage(payload, response.status));
  return payload as T;
}

function errorMessage(payload: unknown, status: number): string {
  if (payload && typeof payload === 'object' && 'error' in payload) {
    const error = payload.error;
    if (typeof error === 'string' && error) return error;
  }
  return `Vault service returned ${status}`;
}
