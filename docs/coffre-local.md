# Coffre local

Tant que la synchronisation est désactivée, RVaultPass se comporte comme un coffre de bureau. Toutes les données vivent dans un fichier `.vault` choisi par l’utilisateur. Aucun service réseau n’est démarré.

## Fichier

Le fichier commence par un en-tête de 96 octets, suivi d’une base SQLite chiffrée par SQLCipher (AES-256-CBC).

| Décalage | Taille | Contenu |
| --- | --- | --- |
| `0x00` | 4 | Magie `KVLT` |
| `0x04` | 1 | Version du format : `0x01` (une seule clé dérivée) ou `0x02` (clés distinctes pour SQLCipher et l’enveloppe) |
| `0x05` | 16 | Sel Argon2id |
| `0x15` | 4 | `mem_cost` Argon2, little-endian, en kio |
| `0x19` | 2 | `time_cost`, little-endian |
| `0x1B` | 2 | `parallelism`, little-endian |
| `0x1D` | 32 | HMAC-SHA256 des octets `0x00..0x1D`. Version 1 : clé Argon2id. Version 2 : clé d’enveloppe |
| `0x3D` | 35 | Réservé, à zéro |

Paramètres par défaut : 65 536 kio (64 Mio), 3 itérations, parallélisme 1. À la lecture, les paramètres sont refusés en dehors de ces bornes : mémoire de 8 Mio à 1 Gio, 1 à 10 itérations, parallélisme de 1 à 8.

La sauvegarde prend un instantané SQLite, l’écrit dans un fichier temporaire, le synchronise sur le disque, puis remplace le coffre. Les générations `.bak.1`, `.bak.2` et `.bak.3` ne tournent qu’après ce remplacement. Un verrou de fichier empêche une seconde ouverture du même coffre.

Si le verrouillage ne peut pas enregistrer, la session reste ouverte. Si le processus s’arrête alors que des modifications ne sont pas encore dans le `.vault`, la base de travail est renommée en `.vault.recovery.db` à côté du coffre. Ce fichier n’est pas réimporté automatiquement.

## Restauration

1. Fermer le bureau et le service, pour relâcher le verrou `.vault.lock`.
2. Pour revenir à une génération précédente, remplacer le `.vault` par une copie de `.bak.1`, `.bak.2` ou `.bak.3`. `.bak.1` est la plus récente.
3. Pour inspecter une session interrompue, ouvrir `.vault.recovery.db` avec la clé SQLCipher de ce coffre. Ne le recopier à la place du `.vault` que si le fichier principal est inutilisable et que la base de récupération s’ouvre.

Le verrouillage efface la clé dérivée de la mémoire du processus.

## Contenu SQLCipher

La base contient les entrées, les dossiers, les étiquettes, la table de liaison entrée-étiquette et un index de recherche FTS5. Le schéma conserve aussi des tables `sync_*` qui ne sont pas lues par le service actuel.

À l’ouverture, une clé aléatoire est enveloppée avec la clé Argon2id et rangée dans `meta`. Le mot de passe maître ne change pas. Le fichier reste ouvrable hors ligne par le bureau ou par le service.

## Interface

Le bureau est une application Tauri. L’interface Preact appelle des commandes Rust. Elle ne détient pas la clé SQLCipher : les secrets affichés passent par ces commandes, puis quittent l’interface au verrouillage.

Commandes principales :

- cycle de vie : créer, ouvrir, verrouiller, enregistrer, savoir si le coffre est ouvert ;
- entrées, dossiers et étiquettes : création, lecture, mise à jour, suppression ;
- recherche plein texte, génération de mot de passe ou de phrase Diceware, code TOTP.

Le bureau, le [client web](v2/web.md) et le [plugin](v2/extension.md) s’appuient sur les mêmes opérations. Le bureau parle à son processus local. Les deux autres passent par le service HTTP.
