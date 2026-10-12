# Modèle de menace

Le service détient le fichier `.vault` et, pendant qu’il est déverrouillé, la clé SQLCipher. Ce n’est pas un service zero-knowledge : un processus compromis à ce moment-là peut lire les entrées.

## Biens à protéger

- Mot de passe maître
- Clé SQLCipher et contenu du coffre
- Jeton de session
- Fichier `.vault` et ses copies `.bak`

## Hypothèses

- Le service tourne sur une machine que vous administrez.
- TLS est terminé par un reverse proxy dès que le client n’est pas sur la même machine.
- Le bureau ne tient pas le même fichier ouvert.
- Le mot de passe maître n’est pas écrit dans les journaux, l’URL ou les arguments.

## Menaces traitées

| Menace | Parade |
| --- | --- |
| Lecture du fichier au repos | SQLCipher et en-tête authentifié par HMAC. |
| Mot de passe deviné en ligne | Argon2id, et la même erreur pour un mauvais mot de passe ou un fichier illisible. |
| Rejeu d’une session après inactivité | Expiration glissante, jeton stocké seulement par son empreinte, verrouillage qui efface la clé. |
| Remplissage du plugin sur un autre site | Le mot de passe n’est demandé qu’après comparaison des origines. |
| Liste d’entrées trop bavarde | Les réponses de liste et de fiche omettent le mot de passe et le secret TOTP. |

## Limites acceptées

- Le serveur déverrouillé peut lire tout le coffre.
- Un jeton volé avant expiration peut lire et modifier le coffre.
- Le plugin et le web font confiance au code du service et au canal TLS.
- Il n’y a qu’un coffre et un mot de passe maître. Le service ne sépare pas plusieurs utilisateurs.
