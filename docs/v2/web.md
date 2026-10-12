# Client web

La page parle au service. Le mot de passe maître est envoyé une fois à `/v1/vault/unlock`, puis oublié. Le jeton reste dans la mémoire de la page.

## Lancer

Deux terminaux. Le service d’abord, avec l’origine de la page :

```bash
cargo run --manifest-path src-tauri/Cargo.toml -p rvault-server -- \
  --vault "$HOME/coffre.vault" \
  --bind 127.0.0.1:8443 \
  --cors-origin http://127.0.0.1:1421
```

Puis la page :

```bash
export PATH="$HOME/.nvm/versions/node/v22.20.0/bin:$PATH"
npm install
npm run dev --workspace rvault-web
```

Ouvrir `http://127.0.0.1:1421`. Vite affiche cette adresse. `http://localhost:1421` fonctionne aussi : le service autorise les deux quand l’une est passée à `--cors-origin`.

Dans la page, l’URL du service est `http://127.0.0.1:8443`.

Arrêter la page : Contrôle-C dans son terminal. Cela ne verrouille pas le coffre. Le verrouillage se fait par le bouton de la page, ou en arrêtant le service.

## Utilisation

1. Indiquer l’URL du service et le mot de passe maître, puis déverrouiller.
2. Chercher, créer ou supprimer une entrée. Le mot de passe et le code TOTP se demandent entrée par entrée.
3. Ajouter un dossier ou une étiquette.
4. Verrouiller. La page oublie le jeton, les listes et les secrets affichés.

L’URL du service est mémorisée dans `localStorage` sous `rvault.serverUrl`. Le mot de passe maître et le jeton n’y sont pas écrits.
