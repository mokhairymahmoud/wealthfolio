import { Module } from "@nestjs/common";

import { ProvidersModule } from "../providers/providers.module";
import { ProviderSyncController } from "./provider-sync.controller";
import { ProviderSyncService } from "./provider-sync.service";

@Module({
  imports: [ProvidersModule],
  controllers: [ProviderSyncController],
  providers: [ProviderSyncService],
})
export class ProviderSyncModule {}
