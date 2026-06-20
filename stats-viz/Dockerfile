# Build stage
FROM node:20-slim AS builder

WORKDIR /app

COPY stats-viz/package*.json ./
RUN npm install

COPY stats-viz/ ./
RUN npm run build

# Production stage
FROM nginx:stable-alpine

COPY --from=builder /app/dist /usr/share/nginx/html

# Default nginx configuration template
RUN mkdir -p /etc/nginx/templates && echo 'server { \
    listen 80; \
    \
    # Docker DNS resolver - re-resolve hostnames every 30s to avoid stale IPs on container restart\
    resolver 127.0.0.11 valid=30s; \
    \
    location / { \
        root /usr/share/nginx/html; \
        index index.html index.htm; \
        try_files $uri $uri/ /index.html; \
    } \
    location /api/ { \
        set $backend "http://feedgen:8000"; \
        rewrite ^/api(/.*)$ $1 break; \
        proxy_pass $backend; \
        proxy_set_header Host $host; \
        proxy_set_header X-Real-IP $remote_addr; \
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for; \
        proxy_set_header X-Forwarded-Proto $scheme; \
        proxy_set_header X-KEY test; \
    } \
}' > /etc/nginx/templates/default.conf.template

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
