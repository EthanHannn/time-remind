# Official website for Time Remind.
#
# The site is plain static HTML/CSS with no build step. The only generated files
# are data/releases.json and the screenshots under assets/, both refreshed by
# scripts/site-releases.mjs when a new release is published.
#
# Build and run locally (context is this directory):
#   docker build -t time-remind-site .
#   docker run --rm -p 8080:80 time-remind-site
#
# On a VPS, put this container behind Caddy, Nginx or Cloudflare and point your
# domain (for example time-remind.dev) at it.

FROM nginx:1.27-alpine

LABEL org.opencontainers.image.title="Time Remind official site" \
      org.opencontainers.image.source="https://github.com/EthanHannn/time-remind" \
      org.opencontainers.image.licenses="MIT"

COPY nginx.conf /etc/nginx/conf.d/default.conf

COPY index.html styles.css /usr/share/nginx/html/
COPY en/ /usr/share/nginx/html/en/
COPY data/ /usr/share/nginx/html/data/
COPY assets/ /usr/share/nginx/html/assets/

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
  CMD wget -q --spider http://127.0.0.1/ || exit 1
