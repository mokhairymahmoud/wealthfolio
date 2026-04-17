FROM node:22-alpine AS base
WORKDIR /app

COPY package.json ./
RUN npm install

COPY tsconfig.json nest-cli.json ./
COPY src ./src

RUN npm run build

EXPOSE 3001

CMD ["npm", "run", "start"]
