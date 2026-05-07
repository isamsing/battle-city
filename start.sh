#!/bin/sh
set -e

# Public IPv4 (dedicated, allocated via fly ips)
PUBLIC_IP="109.105.216.113"

echo "Starting coturn with external-ip=$PUBLIC_IP"

# Start coturn in background
# Listen on 0.0.0.0 (Fly forwards to whatever internal IP it uses)
# external-ip tells coturn what public IP to advertise in TURN allocations
turnserver -c /etc/turnserver.conf \
  --external-ip="$PUBLIC_IP" \
  --listening-ip=0.0.0.0 \
  --relay-ip=0.0.0.0 &

# Run matchbox_server as foreground (container lifecycle tied to this)
exec matchbox_server
