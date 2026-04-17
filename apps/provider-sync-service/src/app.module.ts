import { Module } from "@nestjs/common";

import { AuthModule } from "./auth/auth.module";
import { ConfigModule } from "./config/config.module";
import { CallbackController, HealthController } from "./health.controller";
import { PowensAdminModule } from "./powens-admin/powens-admin.module";
import { ProviderSyncModule } from "./provider-sync/provider-sync.module";

@Module({
  imports: [ConfigModule, AuthModule, PowensAdminModule, ProviderSyncModule],
  controllers: [HealthController, CallbackController],
})
export class AppModule {}
