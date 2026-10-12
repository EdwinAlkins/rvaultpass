import type { ComponentChildren, TargetedEvent } from 'preact';
import { useId } from 'preact/hooks';
import { LockIcon } from './icons';

const AVATAR_COLORS = ['#0c7a62', '#1d6f8a', '#3d6b4f', '#8a5a2b', '#5c4d8a', '#8a3d55'];

/** Ton visuel d'un bouton. */
export type ButtonTone = 'primary' | 'ghost' | 'danger';

/** Taille d'un bouton. */
export type ButtonSize = 'md' | 'sm';

/** Ton d'un message affiché sous un formulaire. */
export type NoticeTone = 'error' | 'ok' | 'info';

export interface MarkProps {
  /** Côté du badge, en pixels. */
  size?: number;
}

/** Badge vert avec le cadenas, utilisé comme marque de RVaultPass. */
export function Mark({ size = 28 }: MarkProps) {
  return (
    <span class="rv-mark" style={{ width: `${size}px`, height: `${size}px` }}>
      <LockIcon size={Math.round(size * 0.58)} />
    </span>
  );
}

export interface BrandProps {
  /** Réduit le badge pour une barre latérale. */
  compact?: boolean;
}

/** Nom du produit précédé de sa marque. */
export function Brand({ compact = false }: BrandProps) {
  return (
    <div class={compact ? 'rv-brand is-compact' : 'rv-brand'}>
      <Mark size={compact ? 26 : 30} />
      <span>RVaultPass</span>
    </div>
  );
}

export interface AvatarProps {
  /** Texte dont la première lettre est affichée. */
  title?: string | null;
  /** Diamètre, en pixels. */
  size?: number;
}

/** Pastille colorée avec l'initiale d'un identifiant. La couleur dépend de cette lettre. */
export function Avatar({ title, size = 36 }: AvatarProps) {
  const letter = (title || '?').trim().charAt(0).toUpperCase() || '?';
  const color = AVATAR_COLORS[[...letter].reduce((sum, char) => sum + char.charCodeAt(0), 0) % AVATAR_COLORS.length];
  return (
    <span
      class="rv-avatar"
      style={{ background: color, width: `${size}px`, height: `${size}px`, fontSize: `${Math.round(size * 0.42)}px` }}
      aria-hidden="true"
    >
      {letter}
    </span>
  );
}

export interface ButtonProps {
  children?: ComponentChildren;
  type?: 'button' | 'submit' | 'reset';
  tone?: ButtonTone;
  size?: ButtonSize;
  disabled?: boolean;
  onClick?: () => void;
}

/** Bouton d'action. Le ton `primary` est vert, `ghost` est discret, `danger` sert à supprimer. */
export function Button({
  children,
  type = 'button',
  tone = 'primary',
  size = 'md',
  disabled = false,
  onClick,
}: ButtonProps) {
  return (
    <button
      type={type}
      class={`rv-btn is-${tone} is-${size}`}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

export interface IconButtonProps {
  /** Nom accessible et infobulle. */
  label: string;
  disabled?: boolean;
  onClick?: () => void;
  children?: ComponentChildren;
}

/** Bouton carré dont le contenu est une icône. */
export function IconButton({ label, disabled = false, onClick, children }: IconButtonProps) {
  return (
    <button type="button" class="rv-icon-btn" aria-label={label} title={label} disabled={disabled} onClick={onClick}>
      {children}
    </button>
  );
}

export interface FieldProps {
  label: string;
  value: string;
  /** Reçoit la valeur courante du champ, pas l'événement du navigateur. */
  onInput: (value: string) => void;
  type?: 'text' | 'url' | 'password' | 'search';
  placeholder?: string;
  autoComplete?: string;
}

/** Champ étiqueté. L'identifiant du contrôle est relié au libellé. */
export function Field({
  label,
  value,
  onInput,
  type = 'text',
  placeholder,
  autoComplete,
}: FieldProps) {
  const id = useId();
  return (
    <label class="rv-field" for={id}>
      <span>{label}</span>
      <input
        id={id}
        type={type}
        value={value}
        placeholder={placeholder}
        autocomplete={autoComplete}
        onInput={(event: TargetedEvent<HTMLInputElement>) => onInput(event.currentTarget.value)}
      />
    </label>
  );
}

export interface SwitchProps {
  label: string;
  checked: boolean;
  /** Reçoit l'état demandé, l'opposé de `checked`. */
  onChange: (checked: boolean) => void;
}

/** Interrupteur à deux états. */
export function Switch({ label, checked, onChange }: SwitchProps) {
  return (
    <div class="rv-switch-row">
      <span>{label}</span>
      <button
        type="button"
        class={checked ? 'rv-switch is-on' : 'rv-switch'}
        role="switch"
        aria-checked={checked}
        aria-label={label}
        onClick={() => onChange(!checked)}
      >
        <span />
      </button>
    </div>
  );
}

export interface NoticeProps {
  children?: ComponentChildren;
  tone?: NoticeTone;
}

/**
 * Message de formulaire.
 *
 * N'affiche rien quand il n'y a pas de contenu.
 * Le ton `error` est annoncé comme une alerte.
 */
export function Notice({ children, tone = 'error' }: NoticeProps) {
  if (!children) return null;
  return <p class={`rv-notice is-${tone}`} role={tone === 'error' ? 'alert' : 'status'}>{children}</p>;
}
