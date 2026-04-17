import { Body, Controller, Post } from "@nestjs/common";

import type { PowensInitTokenRequestDto, PowensRenewTokenRequestDto } from "./powens-admin.dto";
import { PowensAdminService } from "./powens-admin.service";

@Controller("admin/powens")
export class PowensAdminController {
  constructor(private readonly powensAdminService: PowensAdminService) {}

  @Post("auth/init")
  async initUserToken(@Body() request?: PowensInitTokenRequestDto) {
    return this.powensAdminService.initUserToken(request);
  }

  @Post("auth/renew")
  async renewUserToken(@Body() request?: PowensRenewTokenRequestDto) {
    return this.powensAdminService.renewUserToken(request);
  }
}
