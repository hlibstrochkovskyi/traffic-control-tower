# --- Stage 1: Build ---
FROM node:20-alpine as builder

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .
# Build production bundle (output will be in dist folder)
RUN npm run build

# --- Stage 2: Serve (Nginx) ---
FROM nginx:alpine

# Copy build artifacts to Nginx static files directory
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy custom Nginx configuration (will be created separately)
# This is needed to prevent Nginx from failing on page refresh (SPA routing)
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]