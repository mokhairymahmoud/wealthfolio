import { Global, Module } from "@nestjs/common";
import { APP_GUARD } from "@nestjs/core";

import { ServiceTokenGuard } from "./service-token.guard";

@Global()
@Module({
  providers: [
    {
      provide: APP_GUARD,
      useClass: ServiceTokenGuard,
    },
  ],
})
export class AuthModule {}
