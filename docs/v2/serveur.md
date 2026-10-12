# Lancer le service

`rvault-server` sert un fichier `.vault`. Il ne crée pas le coffre : créez-le avec `npm run tauri dev`, verrouillez-le, puis passez son chemin à `--vault`. Le bureau ne doit plus tenir ce fichier ouvert.

```bash
cargo run --manifest-path src-tauri/Cargo.toml -p rvault-server -- \
  --vault "$HOME/coffre.vault" \
  --bind 127.0.0.1:8443 \
  --cors-origin http://127.0.0.1:1421 \
  --idle-lock 900
```

Le fichier doit exister. Sinon le processus quitte avec `Vault file not found`.

## Arguments

| Argument | Obligatoire | Défaut | Rôle |
| --- | --- | --- | --- |
| `--vault FILE` | oui | — | Fichier `.vault` à ouvrir au déverrouillage. |
| `--bind ADDR` | non | `127.0.0.1:8443` | Adresse d’écoute. |
| `--cors-origin URL` | non | aucune | Origine exacte autorisée pour le client web. Répétable. |
| `--idle-lock SECONDS` | non | `900` | Inactivité avant reverrouillage. Au moins 1. |
| `--help` | non | — | Aide, code de sortie 2. |

Un argument inconnu quitte avec le code 2. Un échec de démarrage quitte avec le code 1.

Contrôle-C reverrouille le coffre et arrête l’écoute.

## Réseau

Le binaire parle HTTP. Placez un reverse proxy devant `--bind` pour terminer TLS. N’envoyez pas le mot de passe maître sur un lien en clair hors de la machine.

Le bind par défaut n’écoute que la boucle locale. Pour un autre poste du réseau privé, liez une adresse de cette interface et protégez le port.

`--cors-origin` doit être l’origine de la page web, par exemple `http://127.0.0.1:1421`. Pour cette adresse de boucle locale, `http://localhost:1421` est autorisée en même temps. Le plugin n’en a pas besoin : il déclare l’hôte dans son manifeste.

## Journaux

Le niveau par défaut est `INFO`. Un échec de déverrouillage est journalisé sans le mot de passe, le jeton ni le contenu des entrées.

## Vérifier

```bash
curl -sS http://127.0.0.1:8443/v1/health
```

Réponse attendue : `{"status":"ok"}`. Les routes du coffre répondent 401 tant que `POST /v1/vault/unlock` n’a pas réussi.
