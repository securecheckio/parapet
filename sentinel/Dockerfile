# Use official Playwright image with Chromium pre-installed
FROM mcr.microsoft.com/playwright:v1.43.0-jammy

# Set working directory
WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci --only=production

# Copy source code
COPY . .

# Build TypeScript
RUN npm run build

# Set entrypoint
ENTRYPOINT ["node", "dist/index.js"]
