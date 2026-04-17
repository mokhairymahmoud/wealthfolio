import { Module } from "@nestjs/common";

import { AppConfigService } from "../config/config.service";
import { FixtureProvider } from "./fixtures/fixture.provider";
import { PowensProvider } from "./powens/powens.provider";
import { ProvidersService } from "./providers.service";

@Module({
  providers: [
    FixtureProvider,
    PowensProvider,
    {
      provide: ProvidersService,
      inject: [AppConfigService, FixtureProvider, PowensProvider],
      useFactory: (
        config: AppConfigService,
        fixtureProvider: FixtureProvider,
        powensProvider: PowensProvider,
      ) => new ProvidersService(config, fixtureProvider, powensProvider),
    },
  ],
  exports: [ProvidersService],
})
export class ProvidersModule {}
