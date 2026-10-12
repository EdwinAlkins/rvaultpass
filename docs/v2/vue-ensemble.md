# Lancer chaque élément

Les commandes partent de la racine du dépôt. Le bureau crée le fichier `.vault`. Le service, le client web et le plugin s’utilisent ensuite, une fois le coffre verrouillé dans le bureau.

Node s’obtient via nvm. Ajoutez-le au `PATH` de chaque terminal qui lance npm :

```bash
export PATH="$HOME/.nvm/versions/node/v22.20.0/bin:$PATH"
```

Une seule installation des paquets suffit :

```bash
npm install
```

## 1. Bureau

Crée le coffre, importe les données, puis le verrouille avant de lancer le service.

```bash
npm run tauri dev
```

Dans l’application : créer ou ouvrir le `.vault`, enregistrer, verrouiller. Tant que cette fenêtre tient le fichier ouvert, ne démarrez pas le service sur le même chemin.

## 2. Service

Remplacez le chemin par celui choisi dans le bureau.

```bash
cargo run --manifest-path src-tauri/Cargo.toml -p rvault-server -- \
  --vault "$HOME/coffre.vault" \
  --bind 127.0.0.1:8443 \
  --cors-origin http://127.0.0.1:1421 \
  --idle-lock 900
```

Vérifier, dans un autre terminal :

```bash
curl -sS http://127.0.0.1:8443/v1/health
```

Réponse attendue : `{"status":"ok"}`. Contrôle-C arrête le service et reverrouille le coffre. Le détail des arguments est dans [Lancer le service](serveur.md).

## 3. Client web

Le service de l’étape 2 doit tourner. Dans un autre terminal :

```bash
export PATH="$HOME/.nvm/versions/node/v22.20.0/bin:$PATH"
npm run dev --workspace rvault-web
```

Ouvrir `http://127.0.0.1:1421`. L’URL du service dans la page est `http://127.0.0.1:8443`. Le mot de passe maître est celui du fichier `.vault`.

## 4. Plugin

Le service doit tourner. Construire le plugin, puis le charger dans Firefox :

```bash
export PATH="$HOME/.nvm/versions/node/v22.20.0/bin:$PATH"
npm run build --workspace rvault-extension
```

Le dossier produit est `apps/extension/dist`. Les étapes Firefox sont dans [Plugin navigateur](extension.md).

## Rôles

| Élément | Commande | Adresse |
| --- | --- | --- |
| Bureau | `npm run tauri dev` | fenêtre de l’application |
| Service | `cargo run … -p rvault-server` | `http://127.0.0.1:8443` |
| Web | `npm run dev --workspace rvault-web` | `http://127.0.0.1:1421` |
| Plugin | `npm run build --workspace rvault-extension` | Firefox, module temporaire |

Le bureau et le service ne doivent pas ouvrir le même `.vault` en même temps.
