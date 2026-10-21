# Boond by XPEHO — architecture

Application de gestion commerciale et de suivi des ressources pour une ESN.
Développement indépendant : aucune ligne de code de BoondManager n'est reprise.
Les fonctionnalités s'inspirent seulement de l'analyse de sa documentation publique.

## Choix techniques

| Besoin | Choix |
|---|---|
| Logique métier et serveur | Rust (édition 2024) |
| Interface | Vue 3 + TypeScript + Vite |
| Persistance | SQLite embarqué (`rusqlite`, moteur compilé dans le binaire) |
| Exécutable pour un utilisateur non technique | Tauri 2 : une application `.app` / `.dmg` (et `.msi` sous Windows) |
| Extension LinkedIn | Extension navigateur Manifest V3 (Chrome, Edge, Brave) |

## Vue d'ensemble

```
┌──────────────── Boond by XPEHO.app (Tauri) ────────────────┐
│  Fenêtre native (WebView)  ──HTTP──▶  Serveur local Rust    │
│  Interface Vue (servie par          (127.0.0.1:47821)       │
│  le serveur, même origine)            │                     │
│                                       ▼                     │
│                         boond-http  (API REST, sessions,    │
│                                      contrôle d'accès)      │
│                         boond-service (cas d'usage, règles, │
│                                      permissions, historique)│
│                         boond-store  (SQLite, migrations)   │
│                         boond-domain (modèle, validations,  │
│                                      matching, dates)       │
└──────────────────────────────────────────────────────────────┘
        ▲ Bearer (jeton d'appairage)
        │
Extension « Ajouter à Boond by XPEHO » (service worker MV3)
```

L'interface et l'extension passent par la même API, donc par les mêmes contrôles de permissions côté serveur.
Le même serveur peut tourner seul (`boond-server`) pour le développement ou, plus tard, pour une équipe sur un serveur partagé.

## Découpage du code

```
crates/
  boond-domain   modèle métier pur : types, validations, permissions, matching, règles de dates
  boond-store    persistance SQLite : schéma, migrations, requêtes
  boond-service  cas d'usage : contrôle des droits, validations, transactions, historique
  boond-http     API REST axum, authentification, protection CSRF/Host, fichiers de l'interface
  boond-server   binaire autonome (développement, mode serveur)
desktop/         application Tauri : démarre le serveur local et ouvre la fenêtre
web/             interface Vue
extension/       extension navigateur LinkedIn
```

Règle de dépendance : `domain` ← `store` ← `service` ← `http` ← (`server`, `desktop`).
Le domaine ne fait aucune entrée/sortie. Le stockage ne contient aucune règle métier.

## Modèle de données

```
Société 1─* Contact 1─* Besoin 1─* Positionnement *─1 Ressource
   │                      │                │                │
   │                      └─(gagné)───────▶ Projet 1─* Prestation *─1 Ressource
   └───────────────────────────────────────┘          └─* Prolongation (signée / prévisionnelle)

Activité (appel, rendez-vous, e-mail, relance, note) → liée à société, contact, besoin,
ressource, positionnement ou projet.
Historique → chaque création, modification, changement d'état et archivage.
Ressource 1─* Contrat RH (CJM éventuel), 1─* Document (CV, dossier de compétences).
```

Listes paramétrables (`ref_items`) : types et secteurs de sociétés, états, types et provenances
des besoins, états des positionnements, types et états des projets, types de prestations,
types de contrat. Chaque élément a une **catégorie** stable (`open`, `won`, `lost`, `regie`…)
qui porte les règles métier, tandis que son libellé reste libre.

### Champs facultatifs

Seuls les champs indispensables sont obligatoires (titre et contact d'un besoin, nom d'une
société, nom d'un contact, identité d'une ressource, société d'un projet, date de début d'une
prestation). L'administrateur peut rendre obligatoires certains autres champs. Le **CJM**
n'apparaît jamais dans cette liste : il reste toujours facultatif.

### Règles métier principales

- Un besoin est rattaché à un contact ; sa société est déduite du contact.
- Un positionnement associe une ressource à un besoin (une seule fois par couple).
  Son CJM est proposé à partir du contrat RH en cours ou du CJM saisi sur la ressource.
- Un positionnement passé dans un état de catégorie « gagné » peut créer un projet
  (ou ajouter une prestation à un projet existant) en reprenant société, contact, dates,
  tarif, jours et CJM. Le comportement (demander, automatique, jamais) se règle dans les réglages.
- Les dates d'un projet qui a des prestations sont calculées : début = plus tôt des débuts,
  fin = plus tard des fins (prolongations signées incluses), fin prévisionnelle = avec les
  prolongations prévisionnelles.
- Aucun module de CRA, frais, commandes, facturation, CA prévisionnel ou marge.

## Sécurité et données personnelles

- Mots de passe hachés avec Argon2id. Sessions par cookie `HttpOnly`, `SameSite=Strict`.
- Toute écriture de l'interface exige l'en-tête `X-Boond-Client` : une page web tierce ne peut
  pas l'envoyer sans pré-vérification CORS, que le serveur refuse.
- Le serveur vérifie l'en-tête `Host` (protection contre le DNS rebinding).
- Permissions vérifiées dans `boond-service`, jamais seulement dans l'interface.
  Les coûts (CJM, contrats RH) forment un domaine de droits séparé : sans droit de lecture,
  l'API ne les renvoie pas.
- Extension : jeton d'appairage aléatoire, stocké haché, révocable, lié à un utilisateur
  et donc à ses droits. L'extension n'appelle l'API que depuis son service worker.
- Données stockées localement dans le dossier applicatif de l'utilisateur (droits `0600`).

## Extension LinkedIn

- Le bouton « Ajouter à Boond by XPEHO » apparaît sur une page de profil ; rien n'est lu
  avant le clic.
- Au clic, l'extension lit uniquement ce qui est affiché à l'écran sur ce profil : prénom, nom,
  poste, société, localisation. Aucune requête vers LinkedIn, aucune navigation, aucun
  traitement en masse.
- Un aperçu modifiable s'affiche, avec la recherche de doublons et le choix de la société.
- L'enregistrement n'a lieu qu'après validation. Tout champ illisible se complète à la main.

## Données de développement

L'application démarre vide (hors listes paramétrables par défaut). Les jeux d'essai utilisés
pendant le développement vivent dans un dossier de données séparé (`BOOND_DATA_DIR`) et sont
explicitement marqués « FICTIF ».
