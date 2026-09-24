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

/** Editable metadata — no password / TOTP secret */
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

/** @deprecated Use EntrySummary — kept alias during migration */
export type VaultEntry = EntrySummary;

export interface VaultFolder {
  id: string;
  name: string;
  parent_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface VaultTag {
  id: string;
  name: string;
}

export interface PasswordGenerationParams {
  length: number;
  use_uppercase: boolean;
  use_lowercase: boolean;
  use_numbers: boolean;
  use_symbols: boolean;
  exclude_ambiguous: boolean;
}

export interface TOTPResult {
  code: string;
  time_remaining: number;
}

export interface ImportSummary {
  imported: number;
  skipped: number;
  errors: string[];
}

export interface CommandResult<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}
