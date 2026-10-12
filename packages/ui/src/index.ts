/**
 * Composants Preact partagés par le client web et l'extension.
 *
 * Les styles s'importent séparément : `@rvaultpass/ui/styles.css`.
 * Les jetons de couleur sont dans `@rvaultpass/ui/tokens.css`.
 */
export { Avatar, Brand, Button, Field, IconButton, Mark, Notice, Switch } from './primitives';
export type { AvatarProps, BrandProps, ButtonProps, ButtonSize, ButtonTone, FieldProps, IconButtonProps, MarkProps, NoticeProps, NoticeTone, SwitchProps } from './primitives';
export {
  CopyIcon,
  DiceIcon,
  EyeIcon,
  FolderIcon,
  GearIcon,
  KeyIcon,
  LockIcon,
  PlusIcon,
  SearchIcon,
  TagIcon,
  UserIcon,
  VaultIcon,
} from './icons';
export type { EyeIconProps, IconProps } from './icons';
export {
  Collection,
  EntryComposer,
  EntryDetail,
  EntryList,
  GeneratorPanel,
  NavItem,
  SearchField,
  Toolbar,
  UnlockScreen,
  Workspace,
} from './vault';
export type {
  CollectionProps,
  EntryComposerProps,
  EntryDetailProps,
  EntryDraft,
  EntryListProps,
  EntryTag,
  GeneratorPanelProps,
  NamedItem,
  NavItemProps,
  SearchFieldProps,
  ToolbarProps,
  UnlockScreenProps,
  VaultEntryView,
  WorkspaceProps,
} from './vault';
