# Sécurité

## Signaler une vulnérabilité

**N'ouvrez pas de ticket public.** Utilisez le signalement privé de GitHub (onglet *Security › Report a
vulnerability* du dépôt) en décrivant le problème et comment le reproduire. Ne joignez jamais de données réelles
(contacts, candidats, mots de passe, jetons).

## Versions prises en charge

Seule la dernière version publiée reçoit les correctifs de sécurité.

## Protections intégrées

- **Authentification** : mots de passe hachés avec Argon2id ; sessions par cookie `HttpOnly`, `SameSite=Strict`,
  `Secure` en HTTPS ; HSTS ; jetons d'extension aléatoires de 256 bits, stockés **hachés**, révocables.
- **Force brute** : 6 échecs de connexion bloquent le compte et l'adresse pendant 15 minutes.
- **Installation** : en mode serveur, la création du premier administrateur exige un code d'installation.
- **Droits côté serveur** : chaque action est contrôlée dans la couche métier, jamais seulement dans l'interface.
  Le coût journalier (CJM) n'est jamais envoyé sans le droit correspondant.
- **Requêtes** : en-tête `Host` vérifié (anti DNS-rebinding), en-tête de protection contre les requêtes inter-sites
  sur toute écriture, aucune requête CORS acceptée, en-têtes de sécurité et CSP stricte.
- **Fichiers** : types de document restreints, taille limitée, noms nettoyés, servis avec une CSP `sandbox`.
- **Extension LinkedIn** : n'agit qu'au clic, lit uniquement le profil affiché, n'accepte que `http://127.0.0.1` ou
  une adresse `https://` (autorisée explicitement, une adresse à la fois).
- **Conteneur** : utilisateur sans privilèges, système de fichiers en lecture seule, aucune capacité Linux.

## Ce dont vous restez responsable

Le HTTPS devant le serveur, les mots de passe et rôles de vos utilisateurs, les sauvegardes et leur stockage, les
mises à jour. Voir [docs/DEPLOY.md](docs/DEPLOY.md).

## Dépôt public : ce qui n'y a pas sa place

Aucun fichier `.env`, base de données, sauvegarde, clé, jeton ou donnée de contact réelle. Le dépôt n'en contient
pas ; `.gitignore` et `.dockerignore` les excluent, et la CI doit rester sans secret autre que le jeton automatique
de GitHub.
