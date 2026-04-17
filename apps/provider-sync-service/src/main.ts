import "reflect-metadata";

import { Logger, ValidationPipe } from "@nestjs/common";
import { NestFactory } from "@nestjs/core";
import { existsSync } from "node:fs";
import { loadEnvFile } from "node:process";
import * as path from "node:path";

import { AppModule } from "./app.module";
import { AppConfigService } from "./config/config.service";

async function bootstrap() {
  const envPath = path.resolve(process.cwd(), ".env");
  if (existsSync(envPath)) {
    loadEnvFile(envPath);
  }
  const app = await NestFactory.create(AppModule, {
    cors: true,
  });
  app.setGlobalPrefix("v1", { exclude: ["/"] });
  app.useGlobalPipes(
    new ValidationPipe({
      transform: true,
      whitelist: false,
      forbidUnknownValues: false,
    }),
  );

  const config = app.get(AppConfigService);
  await app.listen(config.port, config.host);
  Logger.log(
    `Provider Sync service listening on http://${config.host}:${config.port}`,
    "Bootstrap",
  );
}

void bootstrap();
