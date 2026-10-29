#!/usr/bin/env sh
# Sends one check-in to the Example Net log on textweaver's own broker, as
# the logger account netcontrol. Placeholder callsign and name. The
# password is asked without echo, or read from TEXTWEAVER_MQTT_PASSWORD.
#
# Across Tailscale, to a plain listener (tw broker --plain --listen
# 100.64.0.10):
tw publish --check-in --host broker.example.ts.net --plain --user netcontrol \
  --topic netlog/checkins \
  --field id=7 --field callsign=N0CALL --field 'name=Ada Example' \
  --field 'location=Portland, OR' --field 'notes=Mobile, portable antenna'

# With TLS, check the broker's key by the pin tw broker printed:
#   tw publish --check-in --host broker.example.ts.net --pin PIN --user netcontrol \
#     --topic netlog/checkins --field callsign=N0CALL
