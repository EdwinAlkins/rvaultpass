# Sécurité

RVaultPass chiffre un coffre local. Le service HTTP, une fois déverrouillé, possède la clé de ce coffre : ce n’est pas un serveur zero-knowledge. La synchronisation chiffrée de bout en bout, si elle est ajoutée plus tard, est une architecture différente.

## Format `.vault`

- Version `1` : la sortie Argon2id sert à la fois à SQLCipher, au HMAC d’en-tête et à l’enveloppe de la clé de données.
- Version `2` : les coffres créés maintenant dérivent deux clés distinctes, une pour SQLCipher et une pour l’enveloppe et le HMAC. Les coffres de version 1 restent ouvrables. Il n’y a pas de rechiffrement silencieux.

Le mot de passe maître d’un nouveau coffre doit contenir au moins 8 caractères. Aucune classe de caractères n’est imposée.

## Session

Une seule session peut ouvrir un coffre en écriture. Le verrou est un verrou du système sur `coffre.vault.lock`, complété par un suivi dans le processus. La sauvegarde suit un instantané SQLite, puis un remplacement du fichier. Le verrouillage n’a lieu qu’après une sauvegarde réussie.

La restauration des fichiers `.bak` et du fichier `.vault.recovery.db` est décrite dans [docs/coffre-local.md](docs/coffre-local.md).

## Limites assumées

Ces points ne sont pas couverts par le code actuel et ne doivent pas être présentés comme faits :

- builds et restauration validés sur Windows, Linux et macOS ;
- tests de bout en bout de l’application bureau ;
- `cargo audit` dans l’intégration continue ;
- revue cryptographique externe.

L’effacement mémoire avec `secrecy` et `zeroize` réduit les copies que le programme contrôle. Il ne garantit pas l’effacement physique dans l’allocateur, le runtime ou le système.
