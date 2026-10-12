# Architecture

```text
Bureau (Tauri)                Client web              Plugin
  ouvre le .vault               HTTP                    HTTP
        |                          \                    /
        |                           \                  /
        +---- un seul processus ----+---- rvault-server ----+
                                    |                        |
                                    +---- fichier .vault ----+
```

Un seul processus détient le fichier. Le bureau extrait la base SQLCipher dans un fichier de travail à côté du `.vault`, tout comme le service. Deux ouvertures simultanées écriraient le même fichier temporaire et les mêmes sauvegardes.

## Où vit la logique

| Élément | Rôle |
| --- | --- |
| `rvault-storage::VaultSession` | Ouverture, verrouillage, sauvegarde atomique. |
| `rvault-storage::operations` | Entrées, dossiers, étiquettes, recherche, générateur, TOTP. |
| `r-vault-pass` | Commandes Tauri. La session est globale dans le processus du bureau. |
| `rvault-server` | Les mêmes opérations derrière HTTP, avec un jeton de session. |
| `@rvaultpass/vault-client` | Client `fetch` utilisé par le web et le plugin. |

Chaque écriture HTTP enregistre le `.vault` avant de répondre. Le verrouillage supprime la base de travail.

Le bureau n’appelle pas le service. Pour utiliser le web ou le plugin, lancez `rvault-server` sur le fichier fermé dans le bureau.
