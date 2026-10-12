# Bureau

Le bureau ouvre le `.vault` dans son propre processus. Créer, modifier et verrouiller un coffre ne passe pas par le service.

## Lancer

Depuis la racine du dépôt :

```bash
export PATH="$HOME/.nvm/versions/node/v22.20.0/bin:$PATH"
npm install
npm run tauri dev
```

La première compilation Rust est longue. L’interface de développement est servie sur `http://127.0.0.1:1420` pour la fenêtre Tauri. Ce n’est pas le client web : celui-ci est sur le port 1421.

Pour un binaire installable :

```bash
npm run tauri build
```

## Avant le service

Enregistrez le coffre, puis verrouillez-le. Le service et le bureau utilisent le même fichier de travail `*.vault.tmp.db` et les mêmes copies `.bak.1` à `.bak.3`. Les ouvrir ensemble mélange ces fichiers.

Le bureau reste l’endroit où créer un coffre neuf et importer un CSV Dashlane. Le service reprend ensuite le chemin de ce fichier.
