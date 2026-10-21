# Boond by XPEHO

Application de gestion commerciale et de suivi des ressources pour une ESN : sociétés, contacts, besoins clients,
candidats et consultants, positionnements, projets et prestations. Développement indépendant, inspiré des
fonctionnalités publiques de BoondManager, sans aucun code propriétaire repris.

- **Rust** : logique métier, base SQLite embarquée, serveur local.
- **Vue 3** : interface.
- **Tauri** : une application de bureau à double-cliquer, sans serveur ni installation à part.
- **Extension navigateur** : « Ajouter à Boond by XPEHO » depuis un profil LinkedIn.
- **Docker** : le même serveur en mode équipe, une base partagée avec comptes et droits.

Hors périmètre volontaire : CRA, frais, commandes, facturation, chiffre d'affaires prévisionnel, marge.

## Utiliser l'application (utilisateur)

1. Ouvrez `Boond-by-XPEHO-….dmg` et glissez **Boond by XPEHO** dans *Applications*.
2. **Première ouverture** : l'application n'est pas signée par un compte développeur Apple. Faites un clic droit sur
   l'icône → **Ouvrir** → **Ouvrir**. Si macOS la bloque encore : *Réglages Système › Confidentialité et sécurité ›
   Ouvrir quand même*. Cette étape n'est nécessaire qu'une fois.
3. Créez le compte administrateur. Vos données restent sur cet ordinateur, dans
   `~/Library/Application Support/fr.xpeho.boond/`.
4. À chaque lancement suivant, l'application s'ouvre directement sur le tableau de bord.

**Sauvegarde** : copiez le dossier `fr.xpeho.boond` (base `boond.sqlite3`, `documents/`, `branding/`).

**Logo XPEHO** : le logo fourni est `web/public/logo.png`. Vous pouvez aussi en téléverser un depuis
*Réglages › Général › Logo XPEHO* sans reconstruire l'application.

## Serveur d'équipe (Docker)

Pour partager une seule base entre plusieurs personnes : `docker compose up -d` derrière un proxy HTTPS, avec un code
d'installation pour le premier administrateur, des comptes et des rôles par personne, des sauvegardes planifiables.
Procédure complète, options HTTPS, sauvegarde/restauration et liste de contrôle : **[docs/DEPLOY.md](docs/DEPLOY.md)**.

## Extension LinkedIn

1. Dans Boond by XPEHO : *Réglages › Extension LinkedIn › Créer un jeton*. Copiez-le (affiché une seule fois).
2. Dans Chrome, Edge ou Brave : `chrome://extensions` → activer le *mode développeur* → *Charger l'extension non
   empaquetée* → dossier `extension/`.
3. Options de l'extension : collez le jeton et l'adresse (`http://127.0.0.1:47821` pour l'application de bureau, ou l'adresse
   `https://…` de votre serveur d'équipe : Chrome demande alors d'autoriser cette adresse, et elle seule), *Tester la connexion*.
4. Sur un profil LinkedIn : icône de l'extension → **Ajouter à Boond by XPEHO** → vérifier l'aperçu → valider.

Principes de l'extension :

- rien n'est lu avant votre clic, et rien n'est enregistré avant la validation de l'aperçu ;
- elle lit uniquement ce qui est déjà affiché sur le profil consulté (prénom, nom, poste, société, localisation) ;
  aucune requête vers LinkedIn, aucune navigation, aucune collecte en masse, aucune protection contournée ;
- les doublons (même profil LinkedIn, même e-mail, même nom) sont signalés avant création ;
- si une donnée n'est pas lisible, le champ reste vide et se complète à la main ;
- l'application (ou le serveur) Boond by XPEHO doit être joignable pendant l'import ;
- le HTTP en clair vers une autre machine est refusé : seules `http://127.0.0.1` et les adresses `https://` sont acceptées.

> LinkedIn modifie régulièrement son code HTML. Le lecteur (`extension/reader.js`) peut donc rater un champ : l'aperçu
> reste éditable, et `node extension/reader.test.mjs` aide à l'adapter.

## Développer

Prérequis : Rust ≥ 1.85, Node ≥ 22.19.

```bash
cd web && npm install && npm run build   # l'interface doit exister avant de compiler le serveur
cd .. && cargo test                      # tests du domaine, de la base et des cas d'usage
```

Boucle de développement avec rechargement de l'interface :

```bash
BOOND_DATA_DIR=/tmp/boond-dev BOOND_PORT=47899 cargo run -p boond-server   # API
cd web && npm run dev                                                      # http://localhost:5173
```

Application de bureau en développement : `cargo run -p boond-desktop`.

Serveur d'équipe en local : `docker compose build` puis voir [docs/DEPLOY.md](docs/DEPLOY.md).

Construire l'application distribuable (macOS) :

```bash
./scripts/package-macos.sh      # → dist/Boond by XPEHO.app et dist/Boond-by-XPEHO-<version>-<arch>.dmg
```

Le `.dmg` produit est propre à l'architecture visée (Apple Silicon par défaut ; `TARGET=x86_64-apple-darwin` pour Intel).

## Publier une version

1. Mettez à jour `version` dans `Cargo.toml` (et `extension/manifest.json` si l'extension change).
2. `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. La CI (`.github/workflows/release.yml`) construit les `.dmg` (Apple Silicon et Intel), publie l'image Docker multi-architecture
   sur `ghcr.io/<propriétaire>/<dépôt>`, joint l'extension zippée et les sommes de contrôle (`SHA256SUMS.txt`) à la release GitHub.

Aucun secret personnalisé n'est nécessaire : la CI utilise uniquement le jeton automatique de GitHub.

Architecture, modèle de données, sécurité : [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Données de développement

L'application démarre **vide** : seuls les listes paramétrables (états, types…) et les rôles par défaut sont créés.
Aucune donnée de démonstration n'est livrée. Les tests créent des données fictives en mémoire, jamais sur disque.
# xpeperience
