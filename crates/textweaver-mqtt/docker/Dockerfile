# Test broker for textweaver-mqtt: Debian 13's own Mosquitto, set up the
# way the live documents guide tells a broker's operator to (a conf.d
# drop-in, a password file made with mosquitto_passwd, a read-only ACL for
# the reader). Test accounts only. The TLS key and certificates are made at
# run time by the test and mounted read-only at /mosquitto/certs; no key is
# ever part of the image or the repository.
FROM debian:trixie

RUN apt-get update \
 && apt-get install -y --no-install-recommends mosquitto mosquitto-clients

COPY netlog.conf /etc/mosquitto/conf.d/netlog.conf
COPY acl /etc/mosquitto/acl

# Test passwords only, for this throwaway broker.
RUN mosquitto_passwd -c -b /etc/mosquitto/passwd logger logger-test-pw \
 && mosquitto_passwd -b /etc/mosquitto/passwd reader reader-test-pw \
 && chown mosquitto:mosquitto /etc/mosquitto/passwd /etc/mosquitto/acl \
 && chmod 0700 /etc/mosquitto/passwd /etc/mosquitto/acl

EXPOSE 1883 8883
CMD ["mosquitto", "-c", "/etc/mosquitto/mosquitto.conf"]
