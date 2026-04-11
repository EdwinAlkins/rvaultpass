import { invoke } from '@tauri-apps/api/core';
import type {
  VaultEntry,
  VaultFolder,
  VaultTag,
  PasswordGenerationParams,
  TOTPResult,
  CommandResult,
} from '../types';

// ============================================================================
// Vault Lifecycle
// ============================================================================

export async function createVault(vaultPath: string, masterPassword: string): Promise<boolean> {
  const result = await invoke<CommandResult<boolean>>('create_vault', {
    vaultPath,
    masterPassword,
  });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to create vault');
  }
  return result.data!;
}

export async function openVault(vaultPath: string, masterPassword: string): Promise<boolean> {
  const result = await invoke<CommandResult<boolean>>('open_vault', {
    vaultPath,
    masterPassword,
  });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to open vault');
  }
  return result.data!;
}

export async function lockVault(): Promise<boolean> {
  const result = await invoke<CommandResult<boolean>>('lock_vault');
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to lock vault');
  }
  return result.data!;
}

export async function saveVault(): Promise<boolean> {
  const result = await invoke<CommandResult<boolean>>('save_vault');
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to save vault');
  }
  return result.data!;
}

export async function isVaultOpen(): Promise<boolean> {
  const result = await invoke<CommandResult<boolean>>('is_vault_open');
  return result.data ?? false;
}

// ============================================================================
// Entry CRUD
// ============================================================================

export async function createEntry(
  folderId: string | null,
  title: string,
  url: string | null,
  username: string | null,
  password: string,
  notes: string | null,
  totpSecret: string | null,
  isFavorite: boolean,
  tagIds: string[] | null,
): Promise<VaultEntry> {
  const result = await invoke<CommandResult<VaultEntry>>('create_entry', {
    folderId,
    title,
    url,
    username,
    password,
    notes,
    totpSecret,
    isFavorite,
    tagIds,
  });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to create entry');
  }
  return result.data!;
}

export async function getAllEntries(): Promise<VaultEntry[]> {
  const result = await invoke<CommandResult<VaultEntry[]>>('get_all_entries');
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to get entries');
  }
  return result.data ?? [];
}

export async function getEntry(entryId: string): Promise<VaultEntry> {
  const result = await invoke<CommandResult<VaultEntry>>('get_entry', { entryId });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to get entry');
  }
  return result.data!;
}

export async function updateEntry(
  entryId: string,
  folderId: string | null,
  title: string | null,
  url: string | null,
  username: string | null,
  password: string | null,
  notes: string | null,
  totpSecret: string | null,
  isFavorite: boolean | null,
  tagIds: string[] | null,
): Promise<VaultEntry> {
  const result = await invoke<CommandResult<VaultEntry>>('update_entry', {
    entryId,
    folderId,
    title,
    url,
    username,
    password,
    notes,
    totpSecret,
    isFavorite,
    tagIds,
  });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to update entry');
  }
  return result.data!;
}

export async function deleteEntry(entryId: string): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('delete_entry', { entryId });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to delete entry');
  }
}

// ============================================================================
// Folder CRUD
// ============================================================================

export async function createFolder(name: string, parentId: string | null): Promise<VaultFolder> {
  const result = await invoke<CommandResult<VaultFolder>>('create_folder', {
    name,
    parentId,
  });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to create folder');
  }
  return result.data!;
}

export async function getAllFolders(): Promise<VaultFolder[]> {
  const result = await invoke<CommandResult<VaultFolder[]>>('get_all_folders');
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to get folders');
  }
  return result.data ?? [];
}

export async function updateFolder(
  folderId: string,
  name: string,
  parentId: string | null,
): Promise<VaultFolder> {
  const result = await invoke<CommandResult<VaultFolder>>('update_folder', {
    folderId,
    name,
    parentId,
  });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to update folder');
  }
  return result.data!;
}

export async function deleteFolder(folderId: string): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('delete_folder', { folderId });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to delete folder');
  }
}

// ============================================================================
// Tag CRUD
// ============================================================================

export async function createTag(name: string): Promise<VaultTag> {
  const result = await invoke<CommandResult<VaultTag>>('create_tag', { name });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to create tag');
  }
  return result.data!;
}

export async function getAllTags(): Promise<VaultTag[]> {
  const result = await invoke<CommandResult<VaultTag[]>>('get_all_tags');
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to get tags');
  }
  return result.data ?? [];
}

export async function deleteTag(tagId: string): Promise<void> {
  const result = await invoke<CommandResult<boolean>>('delete_tag', { tagId });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to delete tag');
  }
}

// ============================================================================
// Search
// ============================================================================

export async function searchEntries(query: string): Promise<VaultEntry[]> {
  const result = await invoke<CommandResult<VaultEntry[]>>('search_entries', { query });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to search entries');
  }
  return result.data ?? [];
}

// ============================================================================
// Password Generator
// ============================================================================

export async function generatePassword(params: PasswordGenerationParams): Promise<string> {
  const result = await invoke<CommandResult<string>>('generate_password', { params });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to generate password');
  }
  return result.data!;
}

export async function generatePassphrase(wordCount: number): Promise<string> {
  const result = await invoke<CommandResult<string>>('generate_passphrase', { wordCount });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to generate passphrase');
  }
  return result.data!;
}

// ============================================================================
// TOTP
// ============================================================================

export async function generateTOTP(secret: string): Promise<TOTPResult> {
  const result = await invoke<CommandResult<TOTPResult>>('generate_totp', { secret });
  if (!result.success || result.error) {
    throw new Error(result.error || 'Failed to generate TOTP');
  }
  return result.data!;
}
