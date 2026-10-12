# Contrat HTTP

Base : l’origine du service, par exemple `http://127.0.0.1:8443`. Les corps sont du JSON. Une erreur est `{ "error": "message" }`.

| Code | Sens |
| --- | --- |
| 204 | Succès sans corps : verrouillage, suppression. |
| 400 | Corps refusé ou entrée sans secret TOTP. |
| 401 | Coffre verrouillé, jeton absent, jeton périmé ou mot de passe refusé. |

`GET /v1/health` et `POST /v1/vault/unlock` ne demandent pas de jeton. Les autres routes exigent :

```http
Authorization: Bearer {token}
```

## Déverrouiller

`POST /v1/vault/unlock`

```json
{ "password": "mot de passe maître" }
```

```json
{ "token": "base64", "expires_at": 1700003600 }
```

`POST /v1/vault/lock` répond 204 et ferme le coffre.

`GET /v1/vault/status`

```json
{ "open": true, "expires_at": 1700003600 }
```

## Entrées

`GET /v1/entries` renvoie des résumés : identifiant, dossier, titre, URL, identifiants, dates, favori, étiquettes, et les drapeaux `has_password`, `has_totp`, `has_notes`. Pas de secret.

`POST /v1/entries`

```json
{
  "title": "Exemple",
  "url": "https://exemple.test",
  "username": "ada",
  "username2": null,
  "username3": null,
  "password": "secret",
  "notes": null,
  "totp_secret": null,
  "folder_id": null,
  "is_favorite": false,
  "tag_ids": []
}
```

`GET /v1/entries/{id}` renvoie la fiche éditable, notes comprises, sans mot de passe ni secret TOTP.

`PUT /v1/entries/{id}` accepte les mêmes champs, tous optionnels. Un champ omis conserve la valeur actuelle. Un mot de passe vide conserve le mot de passe actuel.

`DELETE /v1/entries/{id}` répond 204.

`GET /v1/entries/{id}/password` renvoie `{ "password": "..." }`.

`GET /v1/entries/{id}/totp` renvoie `{ "code": "123456", "time_remaining": 12 }`.

`GET /v1/search?q=exemple` renvoie les mêmes résumés que la liste. La recherche est celle du coffre (FTS5).

## Dossiers et étiquettes

`GET /v1/folders` et `POST /v1/folders` avec `{ "name": "...", "parent_id": null }`.

`PUT /v1/folders/{id}` avec le même corps. `DELETE /v1/folders/{id}` détache les entrées du dossier puis supprime le dossier.

`GET /v1/tags` et `POST /v1/tags` avec `{ "name": "..." }`. `DELETE /v1/tags/{id}`.

## Génération

`POST /v1/passwords/generate`

```json
{
  "length": 20,
  "use_uppercase": true,
  "use_lowercase": true,
  "use_numbers": true,
  "use_symbols": true,
  "exclude_ambiguous": true
}
```

Réponse : `{ "password": "..." }`. La longueur va de 4 à 1024.

`POST /v1/passphrases/generate` avec `{ "word_count": 6 }` répond `{ "passphrase": "..." }`. Le nombre de mots va de 3 à 20.

Ces deux routes exigent une session ouverte.
