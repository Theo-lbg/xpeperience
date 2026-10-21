# Déployer Boond by XPEHO pour une équipe (Docker)

Deux façons d'utiliser Boond by XPEHO :

| | Application de bureau | Serveur d'équipe (ce document) |
|---|---|---|
| Données | sur l'ordinateur de chacun | **une base partagée** sur votre serveur |
| Utilisateurs | un seul poste | plusieurs comptes, avec rôles et droits |
| Accès | fenêtre de l'application | navigateur, via une adresse HTTPS |
| Extension LinkedIn | adresse `http://127.0.0.1:47821` | adresse HTTPS du serveur |

## Ce qui est dans le dépôt, et ce qui reste à votre charge

**Dans le dépôt** (public, sans aucun secret) : le code, le `Dockerfile`, `docker-compose.yml`, un exemple de proxy
HTTPS (`docker-compose.caddy.yml`, `deploy/Caddyfile`), `.env.example`, le script de sauvegarde.

**À faire chez vous** : choisir le nom de domaine, créer votre `.env`, ouvrir les ports du proxy, planifier et
exporter les sauvegardes, appliquer les mises à jour. Rien de tout cela n'est stocké dans le dépôt.

## Prérequis

- Une machine avec Docker et Docker Compose v2.
- Un nom de domaine (ou un accès privé de type Tailscale) : l'extension LinkedIn et le cookie de session exigent
  **HTTPS**. Le serveur refuse d'ailleurs de démarrer ouvert au réseau sans `BOOND_PUBLIC_URL`.

## Mise en route

```bash
git clone https://github.com/OWNER/REPO.git boond && cd boond      # ou téléchargez docker-compose.yml, .env.example, deploy/
cp .env.example .env
$EDITOR .env          # renseignez BOOND_PUBLIC_URL (https://…) et BOOND_DOMAIN
docker compose pull   # image publiée par la release (ou : docker compose build)
docker compose up -d
docker compose logs boond | grep -i "code d'installation"
```

1. Ouvrez l'adresse publique : l'écran **Bienvenue** demande le *code d'installation* affiché dans les journaux.
2. Créez le compte administrateur. Le code ne sert plus ensuite.
3. Dans *Réglages › Utilisateurs*, créez un compte par personne, avec le rôle adapté. N'utilisez pas le compte
   administrateur au quotidien.

> Le code d'installation évite qu'un inconnu qui trouverait l'adresse avant vous ne crée le compte administrateur.
> Vous pouvez aussi le fixer vous-même avec `BOOND_SETUP_CODE`.

## HTTPS : trois options

Le conteneur n'écoute que sur `127.0.0.1:8080` de la machine. Placez un proxy HTTPS devant.

### A. Caddy fourni (le plus simple, domaine public)

Le domaine doit pointer vers la machine, ports 80 et 443 ouverts. Le certificat Let's Encrypt est automatique.

```bash
docker compose -f docker-compose.yml -f docker-compose.caddy.yml up -d
```

### B. Vous avez déjà un proxy (Traefik, Nginx, Nginx Proxy Manager…)

Faites-le pointer vers `127.0.0.1:8080` (ou joignez-le au réseau Docker et ciblez `boond:8080`) et **transmettez**
l'en-tête `Host` d'origine et `X-Forwarded-For`. Exemple Nginx :

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-For $remote_addr;
    client_max_body_size 16m;
}
```

> `X-Forwarded-For` doit être **remplacé** par votre proxy (comme ci-dessus), pas complété, sans quoi un client
> pourrait falsifier son adresse. Le serveur lit le dernier maillon de la chaîne.

### C. Sans ouvrir de port : réseau privé

[Tailscale](https://tailscale.com) (`tailscale serve`/HTTPS) ou un tunnel Cloudflare donnent une adresse HTTPS sans
exposer votre mini PC sur Internet. Renseignez alors cette adresse dans `BOOND_PUBLIC_URL`.

Un simple `http://192.168.x.x` sur le réseau local ne convient pas : mots de passe en clair, et l'extension le refuse.

## Variables d'environnement

| Variable | Rôle |
|---|---|
| `BOOND_PUBLIC_URL` | Adresse publique en `https://`. **Obligatoire.** Sécurise le cookie et active HSTS. |
| `BOOND_TRUST_PROXY` | `1` (défini par le compose) : lit l'adresse réelle du client dans `X-Forwarded-For`. À n'activer que derrière un proxy de confiance. |
| `BOOND_SETUP_CODE` | Code pour créer le premier administrateur (généré et affiché s'il est vide). |
| `BOOND_ALLOWED_HOSTS` | Noms d'hôte supplémentaires acceptés, séparés par des virgules. |
| `BOOND_DOMAIN` | Domaine, pour `docker-compose.caddy.yml` uniquement. |
| `BOOND_HOST_PORT` | Port local exposé (défaut 8080). |

## Sauvegarde et restauration

Les données vivent dans le volume Docker `boond_boond-data` : base SQLite, documents (CV…), logo.

```bash
./deploy/backup.sh /chemin/vers/sauvegardes      # copie cohérente, sans arrêter le serveur
```

Planifiez-la (cron, chaque nuit) et **copiez les sauvegardes ailleurs** : elles contiennent des données personnelles.

Restaurer une sauvegarde :

```bash
docker compose down
docker run --rm -v boond_boond-data:/data -v "$PWD/sauvegardes/boond-AAAAMMJJ-HHMMSS":/backup:ro \
  debian:bookworm-slim sh -c 'rm -rf /data/* && cp -a /backup/. /data/ && chown -R 10001:10001 /data'
docker compose up -d
```

## Mettre à jour

```bash
./deploy/backup.sh /chemin/vers/sauvegardes     # toujours avant une mise à jour
docker compose pull && docker compose up -d
```

La base se met à jour automatiquement au démarrage.

## Sécurité : liste de contrôle

- [ ] Accès **uniquement en HTTPS** ; le port 8080 n'est pas exposé à Internet (le compose le lie à `127.0.0.1`).
- [ ] Mots de passe longs et uniques ; un compte par personne ; rôles au plus juste (*Réglages › Rôles et droits*).
- [ ] `.env` jamais copié dans un dépôt, un ticket ou une capture d'écran.
- [ ] Sauvegardes chiffrées ou stockées sur un support protégé, testées au moins une fois.
- [ ] Mises à jour régulières de l'image et du système hôte.
- [ ] Pare-feu : seuls 80/443 du proxy sont ouverts.
- [ ] Les jetons de l'extension sont révoqués quand un poste est perdu ou qu'une personne part (*Réglages › Extension LinkedIn*).

Protections intégrées : voir [SECURITY.md](../SECURITY.md).

## Dépannage

| Symptôme | Piste |
|---|---|
| Le conteneur redémarre : « BOOND_PUBLIC_URL » | Renseignez l'adresse publique dans `.env`. |
| « Hôte non autorisé » (421) | Le proxy ne transmet pas l'en-tête `Host` d'origine, ou l'adresse ne correspond pas à `BOOND_PUBLIC_URL`. |
| Impossible de rester connecté | L'accès se fait en HTTP alors que `BOOND_PUBLIC_URL` est en HTTPS (cookie `Secure`). Utilisez l'adresse HTTPS. |
| « Trop de tentatives » | 6 échecs de connexion bloquent le compte et l'adresse 15 min. Vérifiez `BOOND_TRUST_PROXY=1` si tout le monde est bloqué en même temps. |
| Code d'installation perdu | `docker compose logs boond`, ou définissez `BOOND_SETUP_CODE` puis `docker compose up -d`. |
| L'extension répond « jeton refusé » | Créez un nouveau jeton dans *Réglages › Extension LinkedIn*. |
