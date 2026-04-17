import { Module } from "@nestjs/common";

import { PowensAdminController } from "./powens-admin.controller";
import { PowensAdminService } from "./powens-admin.service";

@Module({
  controllers: [PowensAdminController],
  providers: [PowensAdminService],
})
export class PowensAdminModule {}
