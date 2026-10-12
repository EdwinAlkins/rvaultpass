/**
 * Composants de présentation du coffre.
 *
 * Ils n'appellent pas le service. L'application qui les utilise charge les données
 * et fournit les actions.
 */
import type { ComponentChildren, TargetedEvent } from 'preact';
import { useState } from 'preact/hooks';
import { CopyIcon, EyeIcon, PlusIcon, SearchIcon } from './icons';
import { Avatar, Button, Field, IconButton, Mark, Notice } from './primitives';
import type { NoticeTone } from './primitives';

/** Élément nommé d'une collection, dossier ou étiquette. */
export interface NamedItem {
  id: string;
  name: string;
}

/** Étiquette affichée sous un identifiant. */
export interface EntryTag {
  id: string;
  name: string;
}

/**
 * Champs nécessaires pour afficher un identifiant.
 *
 * Les secrets n'y figurent pas : le mot de passe et le code TOTP sont fournis à part.
 */
export interface VaultEntryView {
  id: string;
  title: string;
  username?: string | null;
  url?: string | null;
  is_favorite?: boolean;
  has_password?: boolean;
  has_totp?: boolean;
  tags?: EntryTag[];
}

/** Valeurs saisies dans le formulaire de création. */
export interface EntryDraft {
  title: string;
  url: string | null;
  username: string | null;
  password: string;
}

export interface UnlockScreenProps {
  /** Adresse du service, conservée par l'application. */
  serverUrl: string;
  onServerUrl: (value: string) => void;
  busy?: boolean;
  message?: string;
  tone?: NoticeTone;
  /**
   * Déverrouille le coffre.
   *
   * Une erreur rejetée est laissée au parent, qui l'affiche dans `message`.
   */
  onUnlock: (password: string) => void | Promise<unknown>;
}

/** Écran de déverrouillage. Le mot de passe est effacé après une tentative. */
export function UnlockScreen({
  serverUrl,
  onServerUrl,
  busy = false,
  message = '',
  tone = 'error',
  onUnlock,
}: UnlockScreenProps) {
  const [password, setPassword] = useState('');
  const [visible, setVisible] = useState(false);

  const submit = async (event: TargetedEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!password || busy) return;
    try {
      await onUnlock(password);
      setPassword('');
    } catch {
      // The parent surfaces the message.
    }
  };

  return (
    <div class="rv-unlock">
      <form class="rv-card" onSubmit={submit}>
        <Mark size={56} />
        <h1>RVaultPass</h1>
        <p class="rv-lead">Saisissez votre mot de passe maître. Il déverrouille le coffre sur le service, puis il est oublié.</p>
        <label class="rv-field">
          <span>Mot de passe maître</span>
          <span class="rv-password">
            <input
              type={visible ? 'text' : 'password'}
              value={password}
              autocomplete="current-password"
              onInput={(event: TargetedEvent<HTMLInputElement>) => setPassword(event.currentTarget.value)}
            />
            <button
              type="button"
              class="rv-eye"
              aria-label={visible ? 'Masquer le mot de passe' : 'Afficher le mot de passe'}
              onClick={() => setVisible((value) => !value)}
            >
              <EyeIcon off={visible} />
            </button>
          </span>
        </label>
        <Field label="Adresse du service" value={serverUrl} onInput={onServerUrl} type="url" autoComplete="url" />
        <Notice tone={tone}>{message}</Notice>
        <Button type="submit" disabled={busy || !password}>
          {busy ? 'Déverrouillage…' : 'Déverrouiller'}
        </Button>
      </form>
    </div>
  );
}

export interface WorkspaceProps {
  /** Contenu de la colonne de navigation. */
  sidebar: ComponentChildren;
  /** Barre au-dessus de la liste. */
  toolbar: ComponentChildren;
  /** Liste des identifiants. */
  list: ComponentChildren;
  /** Panneau de détail, à droite. */
  detail: ComponentChildren;
}

/** Grille à trois colonnes : navigation, liste, détail. */
export function Workspace({ sidebar, toolbar, list, detail }: WorkspaceProps) {
  return (
    <div class="rv-workspace">
      <aside class="rv-sidebar">{sidebar}</aside>
      <section class="rv-main">
        {toolbar}
        <div class="rv-list">{list}</div>
      </section>
      <aside class="rv-detail">{detail}</aside>
    </div>
  );
}

export interface ToolbarProps {
  title: ComponentChildren;
  search?: ComponentChildren;
  children?: ComponentChildren;
}

/** Titre de la vue, actions, puis champ de recherche. */
export function Toolbar({ title, search, children }: ToolbarProps) {
  return (
    <header class="rv-toolbar">
      <div class="rv-toolbar-row">
        <h1>{title}</h1>
        <div class="rv-toolbar-actions">{children}</div>
      </div>
      {search}
    </header>
  );
}

export interface SearchFieldProps {
  value: string;
  onInput: (value: string) => void;
  placeholder?: string;
}

/** Champ de recherche avec une loupe. Le libellé est lisible par un lecteur d'écran. */
export function SearchField({ value, onInput, placeholder = 'Rechercher un identifiant' }: SearchFieldProps) {
  return (
    <label class="rv-search">
      <span class="rv-sr">Recherche</span>
      <SearchIcon />
      <input
        type="search"
        value={value}
        placeholder={placeholder}
        onInput={(event: TargetedEvent<HTMLInputElement>) => onInput(event.currentTarget.value)}
      />
    </label>
  );
}

export interface NavItemProps {
  icon?: ComponentChildren;
  label: string;
  active?: boolean;
  onClick?: () => void;
}

/** Ligne de navigation. `active` la marque comme la vue courante. */
export function NavItem({ icon, label, active = false, onClick }: NavItemProps) {
  return (
    <button type="button" class={active ? 'rv-nav is-active' : 'rv-nav'} onClick={onClick}>
      {icon}
      <span>{label}</span>
    </button>
  );
}

export interface CollectionProps {
  title: string;
  /** Pictogramme répété devant chaque élément. */
  icon?: ComponentChildren;
  items: NamedItem[];
  selectedId?: string | null;
  onSelect: (item: NamedItem) => void;
  /**
   * Crée un élément à partir du nom saisi.
   *
   * Une erreur rejetée laisse le formulaire ouvert.
   */
  onAdd: (name: string) => void | Promise<unknown>;
  /** Texte affiché quand `items` est vide. */
  empty: string;
}

/** Liste repliable de dossiers ou d'étiquettes, avec un ajout par nom. */
export function Collection({ title, icon, items, selectedId, onSelect, onAdd, empty }: CollectionProps) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [pending, setPending] = useState(false);

  const submit = async (event: TargetedEvent<HTMLFormElement>) => {
    event.preventDefault();
    const value = name.trim();
    if (!value || pending) return;
    setPending(true);
    try {
      await onAdd(value);
      setName('');
      setOpen(false);
    } catch {
      // The parent surfaces the message.
    } finally {
      setPending(false);
    }
  };

  return (
    <section class="rv-collection">
      <header>
        <span>{title}</span>
        <IconButton label={`Ajouter dans ${title}`} onClick={() => setOpen((value) => !value)}>
          <PlusIcon />
        </IconButton>
      </header>
      {open ? (
        <form class="rv-inline" onSubmit={submit}>
          <input
            value={name}
            placeholder="Nom"
            onInput={(event: TargetedEvent<HTMLInputElement>) => setName(event.currentTarget.value)}
          />
          <Button type="submit" size="sm" disabled={pending || !name.trim()}>OK</Button>
        </form>
      ) : null}
      {items.length === 0 ? <p class="rv-muted">{empty}</p> : null}
      <ul>
        {items.map((item) => (
          <li key={item.id}>
            <NavItem
              icon={icon}
              label={item.name}
              active={item.id === selectedId}
              onClick={() => onSelect(item)}
            />
          </li>
        ))}
      </ul>
    </section>
  );
}

export interface EntryListProps {
  entries: VaultEntryView[];
  selectedId?: string | null;
  onSelect?: (entry: VaultEntryView) => void;
  /** Texte affiché quand `entries` est vide. */
  empty: string;
  /** Actions propres à l'application, par exemple copier ou remplir. */
  action?: (entry: VaultEntryView) => ComponentChildren;
}

/** Liste d'identifiants. Un clic sur la ligne appelle `onSelect`, pas `action`. */
export function EntryList({ entries, selectedId, onSelect, empty, action }: EntryListProps) {
  if (entries.length === 0) {
    return (
      <div class="rv-empty">
        <p>{empty}</p>
      </div>
    );
  }

  return (
    <div class="rv-entries">
      {entries.map((entry) => (
        <article
          key={entry.id}
          class={entry.id === selectedId ? 'rv-entry is-selected' : 'rv-entry'}
          onClick={() => onSelect?.(entry)}
        >
          <Avatar title={entry.title} />
          <div class="rv-entry-main">
            <h2>
              {entry.title || 'Sans titre'}
              {entry.is_favorite ? <span class="rv-star" aria-label="Favori">★</span> : null}
            </h2>
            <p>{entry.username || hostOf(entry.url) || 'Sans identifiant'}</p>
            {entry.tags?.length ? (
              <div class="rv-tags">
                {entry.tags.map((tag) => <span key={tag.id}>{tag.name}</span>)}
              </div>
            ) : null}
          </div>
          {action ? (
            <div class="rv-entry-actions" onClick={(event: TargetedEvent<HTMLDivElement>) => event.stopPropagation()}>
              {action(entry)}
            </div>
          ) : null}
        </article>
      ))}
    </div>
  );
}

export interface EntryDetailProps {
  entry?: VaultEntryView | null;
  /** Mot de passe déjà révélé, ou vide tant qu'il ne l'est pas. */
  secret?: string | null;
  /** Code TOTP déjà demandé, ou vide tant qu'il ne l'est pas. */
  code?: string | null;
  onRevealPassword?: () => void;
  onRevealTotp?: () => void;
  onDelete?: () => void;
  /** Reçoit le texte à copier. L'application écrit dans le presse-papiers. */
  onCopy?: (value: string) => void;
}

/** Détail d'un identifiant. Sans `entry`, le panneau invite à en choisir un. */
export function EntryDetail({
  entry,
  secret,
  code,
  onRevealPassword,
  onRevealTotp,
  onDelete,
  onCopy,
}: EntryDetailProps) {
  if (!entry) {
    return <div class="rv-detail-empty">Sélectionnez un identifiant pour voir son détail.</div>;
  }

  return (
    <div class="rv-detail-body">
      <header class="rv-detail-head">
        <Avatar title={entry.title} size={48} />
        <div>
          <h2>{entry.title || 'Sans titre'}</h2>
          {entry.url ? <p>{hostOf(entry.url) || entry.url}</p> : null}
        </div>
      </header>
      <div class="rv-detail-card">
        <DetailValue label="Identifiant" value={entry.username} onCopy={onCopy} />
        <DetailValue label="Site web" value={entry.url} href={entry.url} onCopy={onCopy} />
        <div class="rv-detail-field">
          <span>Mot de passe</span>
          <div class="rv-detail-line">
            <strong class="rv-secret">{secret || (entry.has_password ? '••••••••••' : '—')}</strong>
            {entry.has_password ? (
              <IconButton label={secret ? 'Mot de passe affiché' : 'Afficher le mot de passe'} onClick={onRevealPassword}>
                <EyeIcon off={Boolean(secret)} />
              </IconButton>
            ) : null}
            {secret ? (
              <IconButton label="Copier le mot de passe" onClick={() => onCopy?.(secret)}>
                <CopyIcon />
              </IconButton>
            ) : null}
          </div>
        </div>
        {entry.has_totp || code ? (
          <div class="rv-detail-field">
            <span>Code à usage unique</span>
            <div class="rv-detail-line">
              <strong class="rv-code">{code || '••• •••'}</strong>
              <Button size="sm" tone="ghost" onClick={onRevealTotp}>{code ? 'Actualiser' : 'Afficher'}</Button>
              {code ? (
                <IconButton label="Copier le code" onClick={() => onCopy?.(code)}>
                  <CopyIcon />
                </IconButton>
              ) : null}
            </div>
          </div>
        ) : null}
      </div>
      <div class="rv-detail-actions">
        <Button size="sm" tone="danger" onClick={onDelete}>Supprimer</Button>
      </div>
    </div>
  );
}

interface DetailValueProps {
  label: string;
  value?: string | null;
  href?: string | null;
  onCopy?: (value: string) => void;
}

function safeHttpHref(value: string | null | undefined): string | null {
  if (!value) return null;
  try {
    const parsed = new URL(value);
    if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') return null;
    return parsed.href;
  } catch {
    return null;
  }
}

function DetailValue({ label, value, href, onCopy }: DetailValueProps) {
  const destination = safeHttpHref(href);
  return (
    <div class="rv-detail-field">
      <span>{label}</span>
      <div class="rv-detail-line">
        {destination ? <a href={destination} target="_blank" rel="noreferrer">{value}</a> : <strong>{value || '—'}</strong>}
        {value && onCopy ? (
          <IconButton label={`Copier ${label.toLowerCase()}`} onClick={() => onCopy(value)}>
            <CopyIcon />
          </IconButton>
        ) : null}
      </div>
    </div>
  );
}

export interface EntryComposerProps {
  busy?: boolean;
  /**
   * Enregistre le brouillon.
   *
   * Une erreur rejetée conserve les champs saisis.
   */
  onSubmit: (draft: EntryDraft) => void | Promise<unknown>;
  onCancel?: () => void;
}

/** Formulaire d'un nouvel identifiant. Les champs sont vidés après un enregistrement réussi. */
export function EntryComposer({ busy = false, onSubmit, onCancel }: EntryComposerProps) {
  const [title, setTitle] = useState('');
  const [url, setUrl] = useState('');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');

  const submit = async (event: TargetedEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (busy) return;
    try {
      await onSubmit({
        title: title.trim() || 'Sans titre',
        url: url.trim() || null,
        username: username.trim() || null,
        password,
      });
      setTitle('');
      setUrl('');
      setUsername('');
      setPassword('');
    } catch {
      // The parent surfaces the message.
    }
  };

  return (
    <form class="rv-detail-body" onSubmit={submit}>
      <header class="rv-detail-head">
        <Avatar title={title || '+'} size={48} />
        <div><h2>Nouvel identifiant</h2></div>
      </header>
      <Field label="Nom" value={title} onInput={setTitle} />
      <Field label="Site web" value={url} onInput={setUrl} type="url" autoComplete="url" />
      <Field label="Identifiant" value={username} onInput={setUsername} autoComplete="username" />
      <Field label="Mot de passe" value={password} onInput={setPassword} type="password" autoComplete="new-password" />
      <div class="rv-detail-actions">
        <Button type="submit" size="sm" disabled={busy}>{busy ? 'Enregistrement…' : 'Enregistrer'}</Button>
        <Button type="button" size="sm" tone="ghost" onClick={onCancel}>Annuler</Button>
      </div>
    </form>
  );
}

export interface GeneratorPanelProps {
  /** Mot de passe déjà généré. Vide tant que le premier n'est pas arrivé. */
  value?: string;
  onGenerate?: () => void;
  onCopy?: () => void;
}

/** Affiche un mot de passe généré et propose d'en demander un autre. */
export function GeneratorPanel({ value, onGenerate, onCopy }: GeneratorPanelProps) {
  return (
    <section class="rv-generator">
      <h2>Générateur</h2>
      <div class="rv-secret-box">
        <code>{value || '…'}</code>
        <IconButton label="Copier le mot de passe" disabled={!value} onClick={onCopy}>
          <CopyIcon />
        </IconButton>
      </div>
      <Button size="sm" onClick={onGenerate}>Générer</Button>
    </section>
  );
}

function hostOf(url?: string | null): string {
  if (!url) return '';
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}
