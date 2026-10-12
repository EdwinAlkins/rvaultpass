# Plugin navigateur

Le plugin est un client Manifest V3 du service. Le service doit être lancé, et le fichier `.vault` ne doit pas être ouvert dans le bureau.

## Construire

Depuis la racine du dépôt :

```bash
export PATH="$HOME/.nvm/versions/node/v22.20.0/bin:$PATH"
npm install
npm run build --workspace rvault-extension
```

Le dossier produit est `apps/extension/dist`. Son `manifest.json` sert à Firefox et à Chrome : la clé `background` déclare `scripts` pour Firefox et `service_worker` pour Chrome, comme le recommande MDN. Reconstruisez après chaque modification des sources, puis rechargez le module.

Le manifeste demande `storage`, `activeTab`, `scripting` et `tabs`, sans accès hôte permanent. Le script de remplissage est injecté seulement quand vous remplissez un formulaire. Le build produit `background.js` et `content.js` comme fichiers autonomes, sans `import`, car le script d’arrière-plan et le script injecté ne chargent pas de modules.

## Ouvrir dans Firefox pour le développement

Firefox 121 ou plus récent. Le module temporaire disparaît à la fermeture du navigateur.

1. Lancez le service, par exemple sur `http://127.0.0.1:8443`. Le plugin n’a pas besoin de `--cors-origin`.
2. Construisez le plugin avec la commande ci-dessus.
3. Dans Firefox, ouvrez `about:debugging`.
4. Dans le menu de gauche, choisissez **Ce Firefox**.
5. Cliquez **Charger un module complémentaire temporaire…**.
6. Sélectionnez `apps/extension/dist/manifest.json`. Firefox charge le dossier du fichier choisi et lit toujours son `manifest.json`.
7. L’icône RVaultPass apparaît dans la barre d’outils. Si elle est dans le menu des extensions, épinglez-la.
8. Clic droit sur l’icône, puis **Gérer l’extension** si vous voulez la page d’options. Vous pouvez aussi cliquer **Inspecter** sur `about:debugging` pour la console du script d’arrière-plan.

Après une reconstruction :

1. Retournez sur `about:debugging`, section **Ce Firefox**.
2. Sur la carte RVaultPass, cliquez **Recharger**.

## Première utilisation

1. Ouvrez le popup depuis l’icône. Saisissez le mot de passe maître, puis déverrouillez. L’adresse du service, `http://127.0.0.1:8443` par défaut, est sous **Adresse du service**, ou dans l’onglet **Réglages**.
2. Les identifiants du site ouvert sont listés sous **Pour ce site**. Un clic sur la ligne remplit le formulaire lorsque l’origine de l’onglet est celle de l’entrée, puis le popup se ferme.
3. Les icônes de la ligne copient l’identifiant ou le mot de passe.
4. L’onglet **Générateur** crée un mot de passe. L’onglet **Réglages** change l’adresse du service et verrouille le coffre.

Verrouiller appelle `POST /v1/vault/lock` et retire le jeton. Ce jeton vit dans `chrome.storage.session`, vidé à la fermeture de Firefox. L’URL du service reste dans `chrome.storage.local`.
