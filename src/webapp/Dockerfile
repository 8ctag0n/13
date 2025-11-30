# ZyberLink Frontend Dockerfile
# Multi-stage build for production

# Stage 1: Build
FROM docker.io/library/node:22-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .

ARG VITE_API_URL=""
ARG VITE_SOLANA_RPC_URL="http://localhost:8899"
ENV VITE_API_URL=$VITE_API_URL
ENV VITE_SOLANA_RPC_URL=$VITE_SOLANA_RPC_URL

RUN npm run build

# Stage 2: Production nginx
FROM docker.io/library/nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html

RUN echo 'server { \
    listen 80; \
    root /usr/share/nginx/html; \
    index index.html; \
    location / { try_files $uri $uri/ /index.html; } \
    location ~* \.(js|css|png|jpg|ico|svg|woff|woff2)$ { \
        expires 1y; \
        add_header Cache-Control "public, immutable"; \
    } \
}' > /etc/nginx/conf.d/default.conf

EXPOSE 80
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD wget -q --spider http://localhost/ || exit 1
CMD ["nginx", "-g", "daemon off;"]
