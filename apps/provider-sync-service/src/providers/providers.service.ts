import { Injectable } from "@nestjs/common";

import { AppConfigService } from "../config/config.service";
import { FixtureProvider } from "./fixtures/fixture.provider";
import { PowensProvider } from "./powens/powens.provider";
import type { AggregationProvider } from "./provider.types";

@Injectable()
export class ProvidersService {
  constructor(
    private readonly config: AppConfigService,
    private readonly fixtureProvider: FixtureProvider,
    private readonly powensProvider: PowensProvider,
  ) {}

  getProvider(): AggregationProvider {
    return this.config.provider === "powens" ? this.powensProvider : this.fixtureProvider;
  }
}
