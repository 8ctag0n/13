# ZyberLink Frontend Dockerfile
# =============================
# Multi-stage build for the Svelte frontend

# Stage 1: Build
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy source files
COPY . .

# Build the app
RUN npm run build

# Stage 2: Development server
FROM node:20-alpine

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies (including dev deps for vite)
RUN npm ci

# Copy source files
COPY . .

# Expose port
EXPOSE 5173

# Set environment defaults
ENV HOST=0.0.0.0

# Run vite dev server (for development)
# For production, use a proper web server like nginx
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]
