import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

const en = {
  translation: {
    // Common
    'app.title': 'RVaultPass',
    'common.save': 'Save',
    'common.cancel': 'Cancel',
    'common.delete': 'Delete',
    'common.edit': 'Edit',
    'common.add': 'Add',
    'common.search': 'Search...',
    'common.close': 'Close',
    'common.copy': 'Copy',
    'common.copied': 'Copied!',
    'common.loading': 'Loading...',
    'common.error': 'Error',
    'common.success': 'Success',

    // Vault Screen
    'vault.welcome': 'Welcome to RVaultPass',
    'vault.description': 'Your local, zero-knowledge password vault',
    'vault.create': 'Create Vault',
    'vault.open': 'Open Vault',
    'vault.file_path': 'Vault File Path',
    'vault.file_path_placeholder': '/path/to/vault.vault',
    'vault.browse': 'Browse...',
    'vault.create_title': 'Create Vault',
    'vault.open_title': 'Open Vault',
    'vault.password': 'Master Password',
    'vault.password_placeholder': 'Enter your master password',
    'vault.confirm_password': 'Confirm Password',
    'vault.confirm_password_placeholder': 'Confirm your master password',
    'vault.creating': 'Creating vault...',
    'vault.opening': 'Opening vault...',
    'vault.passwords_mismatch': 'Passwords do not match',
    'vault.empty_password': 'Password cannot be empty',
    'vault.empty_path': 'File path cannot be empty',

    // Main Layout
    'sidebar.entries': 'All Entries',
    'sidebar.favorites': 'Favorites',
    'sidebar.folders': 'Folders',
    'sidebar.tags': 'Tags',
    'sidebar.add_folder': 'Add Folder',
    'sidebar.add_tag': 'Add Tag',
    'sidebar.new_folder_name': 'New Folder',
    'sidebar.new_tag_name': 'New Tag',

    // Entry List
    'entry.list.empty': 'No entries yet',
    'entry.list.empty_desc': 'Create your first password entry',
    'entry.add': 'Add Entry',
    'entry.edit': 'Edit Entry',
    'entry.delete_confirm': 'Are you sure you want to delete this entry?',
    'entry.no_password': 'No password',

    // Entry Form
    'entry.title': 'Title',
    'entry.title_placeholder': 'Example: Gmail',
    'entry.url': 'URL',
    'entry.url_placeholder': 'https://example.com',
    'entry.username': 'Username',
    'entry.username_placeholder': 'user@example.com',
    'entry.password': 'Password',
    'entry.notes': 'Notes',
    'entry.notes_placeholder': 'Additional notes...',
    'entry.totp_secret': 'TOTP Secret',
    'entry.totp_secret_placeholder': 'Base32 secret or URI',
    'entry.folder': 'Folder',
    'entry.tags': 'Tags',
    'entry.is_favorite': 'Mark as favorite',
    'entry.generate_password': 'Generate Password',

    // Password Generator
    'generator.title': 'Password Generator',
    'generator.length': 'Length',
    'generator.uppercase': 'Uppercase (A-Z)',
    'generator.lowercase': 'Lowercase (a-z)',
    'generator.numbers': 'Numbers (0-9)',
    'generator.symbols': 'Symbols (!@#$)',
    'generator.exclude_ambiguous': 'Exclude ambiguous (l, 1, O, 0)',
    'generator.generate': 'Generate',
    'generator.passphrase': 'Passphrase',
    'generator.word_count': 'Word Count',
    'generator.generate_passphrase': 'Generate Passphrase',

    // TOTP
    'totp.title': 'TOTP Code',
    'totp.time_remaining': 'Refreshes in {{seconds}}s',

    // Theme
    'theme.dark': 'Dark Mode',
    'theme.light': 'Light Mode',

    // Actions
    'action.lock': 'Lock Vault',
    'action.save': 'Save Vault',
    'action.locked': 'Vault Locked',

    // Notifications
    'notif.entry_created': 'Entry created',
    'notif.entry_updated': 'Entry updated',
    'notif.entry_deleted': 'Entry deleted',
    'notif.folder_created': 'Folder created',
    'notif.tag_created': 'Tag created',
    'notif.vault_saved': 'Vault saved',
    'notif.vault_locked': 'Vault locked',
    'notif.copied_clipboard': 'Copied to clipboard',
    'notif.clipboard_warning': 'Warning: clipboard contents will clear in 30s',
  },
};

const fr = {
  translation: {
    // Common
    'app.title': 'RVaultPass',
    'common.save': 'Sauvegarder',
    'common.cancel': 'Annuler',
    'common.delete': 'Supprimer',
    'common.edit': 'Modifier',
    'common.add': 'Ajouter',
    'common.search': 'Rechercher...',
    'common.close': 'Fermer',
    'common.copy': 'Copier',
    'common.copied': 'Copié !',
    'common.loading': 'Chargement...',
    'common.error': 'Erreur',
    'common.success': 'Succès',

    // Vault Screen
    'vault.welcome': 'Bienvenue sur RVaultPass',
    'vault.description': 'Votre coffre-fort local zero-knowledge',
    'vault.create': 'Créer un coffre',
    'vault.open': 'Ouvrir un coffre',
    'vault.file_path': 'Chemin du fichier',
    'vault.file_path_placeholder': '/chemin/vers/coffre.vault',
    'vault.browse': 'Parcourir...',
    'vault.create_title': 'Créer un coffre',
    'vault.open_title': 'Ouvrir un coffre',
    'vault.password': 'Mot de passe maître',
    'vault.password_placeholder': 'Entrez votre mot de passe maître',
    'vault.confirm_password': 'Confirmer le mot de passe',
    'vault.confirm_password_placeholder': 'Confirmez votre mot de passe maître',
    'vault.creating': 'Création du coffre...',
    'vault.opening': 'Ouverture du coffre...',
    'vault.passwords_mismatch': 'Les mots de passe ne correspondent pas',
    'vault.empty_password': 'Le mot de passe ne peut pas être vide',
    'vault.empty_path': 'Le chemin du fichier ne peut pas être vide',

    // Main Layout
    'sidebar.entries': 'Toutes les entrées',
    'sidebar.favorites': 'Favoris',
    'sidebar.folders': 'Dossiers',
    'sidebar.tags': 'Étiquettes',
    'sidebar.add_folder': 'Ajouter dossier',
    'sidebar.add_tag': 'Ajouter étiquette',
    'sidebar.new_folder_name': 'Nouveau dossier',
    'sidebar.new_tag_name': 'Nouvelle étiquette',

    // Entry List
    'entry.list.empty': 'Aucune entrée',
    'entry.list.empty_desc': 'Créez votre première entrée',
    'entry.add': 'Ajouter entrée',
    'entry.edit': 'Modifier entrée',
    'entry.delete_confirm': 'Êtes-vous sûr de vouloir supprimer cette entrée ?',
    'entry.no_password': 'Pas de mot de passe',

    // Entry Form
    'entry.title': 'Titre',
    'entry.title_placeholder': 'Exemple : Gmail',
    'entry.url': 'URL',
    'entry.url_placeholder': 'https://exemple.com',
    'entry.username': "Nom d'utilisateur",
    'entry.username_placeholder': 'user@exemple.com',
    'entry.password': 'Mot de passe',
    'entry.notes': 'Notes',
    'entry.notes_placeholder': 'Notes additionnelles...',
    'entry.totp_secret': 'Secret TOTP',
    'entry.totp_secret_placeholder': 'Secret Base32 ou URI',
    'entry.folder': 'Dossier',
    'entry.tags': 'Étiquettes',
    'entry.is_favorite': 'Marquer comme favori',
    'entry.generate_password': 'Générer mot de passe',

    // Password Generator
    'generator.title': 'Générateur de mot de passe',
    'generator.length': 'Longueur',
    'generator.uppercase': 'Majuscules (A-Z)',
    'generator.lowercase': 'Minuscules (a-z)',
    'generator.numbers': 'Chiffres (0-9)',
    'generator.symbols': 'Symboles (!@#$)',
    'generator.exclude_ambiguous': 'Exclure ambigus (l, 1, O, 0)',
    'generator.generate': 'Générer',
    'generator.passphrase': 'Phrase de passe',
    'generator.word_count': 'Nombre de mots',
    'generator.generate_passphrase': 'Générer phrase de passe',

    // TOTP
    'totp.title': 'Code TOTP',
    'totp.time_remaining': 'Rafraîchit dans {{seconds}}s',

    // Theme
    'theme.dark': 'Mode sombre',
    'theme.light': 'Mode clair',

    // Actions
    'action.lock': 'Verrouiller',
    'action.save': 'Sauvegarder',
    'action.locked': 'Coffre verrouillé',

    // Notifications
    'notif.entry_created': 'Entrée créée',
    'notif.entry_updated': 'Entrée mise à jour',
    'notif.entry_deleted': 'Entrée supprimée',
    'notif.folder_created': 'Dossier créé',
    'notif.tag_created': 'Étiquette créée',
    'notif.vault_saved': 'Coffre sauvegardé',
    'notif.vault_locked': 'Coffre verrouillé',
    'notif.copied_clipboard': 'Copié dans le presse-papiers',
    'notif.clipboard_warning': 'Attention : le contenu sera effacé dans 30s',
  },
};

i18n.use(initReactI18next).init({
  resources: {
    en,
    fr,
  },
  lng: 'en',
  fallbackLng: 'en',
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;
