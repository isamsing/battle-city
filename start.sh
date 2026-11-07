#!/bin/sh
set -e

# Public IPv4 (dedicated, allocated via fly ips)
PUBLIC_IP="109.105.216.113"

echo "Starting coturn with external-ip=$PUBLIC_IP"

TURN_USER="${TURN_USER:-battlecity}"
TURN_PASS="${TURN_PASS:-tankbattle2026}"
if [ "$TURN_PASS" = "tankbattle2026" ]; then
  echo "WARNING: using the default TURN password; run: fly secrets set TURN_PASS=..." >&2
fi

# Start coturn in background
# Listen on 0.0.0.0 (Fly forwards to whatever internal IP it uses)
# external-ip tells coturn what public IP to advertise in TURN allocations
turnserver -c /etc/turnserver.conf \
  --external-ip="$PUBLIC_IP" \
  --listening-ip=0.0.0.0 \
  --relay-ip=0.0.0.0 \
  --user="$TURN_USER:$TURN_PASS" &

# Run matchbox_server as foreground (container lifecycle tied to this)
exec matchbox_server
