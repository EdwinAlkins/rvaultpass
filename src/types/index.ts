export interface VaultEntry {
  id: string;
  folder_id: string | null;
  title: string;
  url: string | null;
  username: string | null;
  password: string;
  notes: string | null;
  totp_secret: string | null;
  created_at: string;
  updated_at: string;
  is_favorite: boolean;
  tags: VaultTag[];
}

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

export interface CommandResult<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}
