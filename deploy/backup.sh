#!/usr/bin/env bash
# Sauvegarde cohérente d'un serveur XpePerience lancé avec docker compose.
#
#   ./deploy/backup.sh [dossier-de-destination]     (défaut : ./sauvegardes)
#
# Crée un dossier horodaté contenant la base (copie cohérente, sans arrêter le serveur), les documents et le logo.
# À planifier avec cron, par exemple chaque nuit :
#   0 3 * * *  cd /chemin/vers/xpeperience && ./deploy/backup.sh /chemin/vers/sauvegardes
# Pensez à copier ces sauvegardes AILLEURS que sur la machine (autre disque, autre site) :
# elles contiennent les données personnelles de vos contacts et candidats.
set -euo pipefail

cd "$(dirname "$0")/.."
DEST="${1:-./sauvegardes}"
STAMP="$(date +%Y%m%d-%H%M%S)"
mkdir -p "$DEST"
DEST="$(cd "$DEST" && pwd)"

# Sauvegarde dans le conteneur, puis transfert par flux tar (fonctionne quel que soit le type de montage).
TMP="/tmp/backup-$STAMP"
docker compose exec -T xpeperience xpeperience-server backup "$TMP" >/dev/null
mkdir -p "$DEST/xpeperience-$STAMP"
docker compose exec -T xpeperience tar -C "$TMP" -cf - . | tar -C "$DEST/xpeperience-$STAMP" -xf -
docker compose exec -T xpeperience rm -rf "$TMP"
chmod -R go-rwx "$DEST/xpeperience-$STAMP"
[ -s "$DEST/xpeperience-$STAMP/xpeperience.sqlite3" ] || { echo "Sauvegarde incomplète : xpeperience.sqlite3 absent" >&2; exit 1; }
echo "Sauvegarde : $DEST/xpeperience-$STAMP"

# Conserve les 14 dernières sauvegardes.
ls -1dt "$DEST"/xpeperience-* 2>/dev/null | tail -n +15 | xargs -r rm -rf
