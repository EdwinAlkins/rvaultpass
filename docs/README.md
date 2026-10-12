# Documentation RVaultPass

La référence d’architecture est [docs/v2/](v2/vue-ensemble.md). `Specifications.md`, à la racine, décrit l’ancienne cible : un coffre uniquement local, sans service HTTP ni extension.

RVaultPass conserve les mots de passe dans un fichier `.vault`. Le bureau ouvre ce fichier localement. Un service optionnel peut ouvrir le même format et exposer les opérations du coffre en HTTP.

Le mot de passe maître ne sert qu’à déverrouiller. Ensuite, les clients présentent un jeton de session. Tant que le coffre est déverrouillé, le processus serveur peut lire les secrets.

## Coffre local

- [Format du fichier `.vault`](coffre-local.md)

## Service

- [Commandes pour lancer chaque élément](v2/vue-ensemble.md)
- [Architecture](v2/architecture.md)
- [Lancer le service](v2/serveur.md)
- [Session et secrets](v2/chiffrement.md)
- [Contrat HTTP](v2/sync-protocol.md)
- [Bureau](v2/desktop.md)
- [Client web](v2/web.md)
- [Plugin navigateur](v2/extension.md)
- [Modèle de menace](v2/threat-model.md)
