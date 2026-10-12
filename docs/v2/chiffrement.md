# Session et secrets

Le fichier `.vault` reste chiffré par SQLCipher. Argon2id dérive la clé depuis le mot de passe maître et le sel de l’en-tête. Le format est décrit dans [le coffre local](../coffre-local.md).

Le service ne reçoit le mot de passe que dans `POST /v1/vault/unlock`. Il ne le journalise pas, ne le met pas dans les arguments du processus et ne le renvoie pas. Un mot de passe faux, un fichier illisible ou un en-tête invalide produisent la même réponse : 401 et `{ "error": "Unlock failed" }`.

Le jeton de session fait 32 octets aléatoires, encodés en base64. Le processus n’en garde que le SHA-256. Chaque requête authentifiée repousse l’expiration de `--idle-lock` secondes. À l’échéance, le service ferme la base de travail et oublie la clé.

Un second déverrouillage avec le bon mot de passe, pendant que le coffre est déjà ouvert, renouvelle le jeton sans rouvrir le fichier. L’ancien jeton cesse de fonctionner. Un mauvais mot de passe dans ce cas répond 401 et laisse la session en cours intacte.

Les listes et la fiche d’une entrée n’incluent ni le mot de passe ni le secret TOTP. Ils indiquent seulement `has_password` et `has_totp`. Le mot de passe s’obtient avec `GET /v1/entries/{id}/password`, le code TOTP avec `GET /v1/entries/{id}/totp`.

Le client web garde le jeton en mémoire de page. Le plugin le garde dans `chrome.storage.session`, vidé à la fermeture du navigateur. Ni l’un ni l’autre n’écrit le mot de passe maître sur le disque.
