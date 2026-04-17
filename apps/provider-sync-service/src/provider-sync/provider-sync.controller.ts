import { BadRequestException, Body, Controller, Get, Param, Post, Query } from "@nestjs/common";

import type { SyncRequestDto } from "./dto";
import { ProviderSyncService } from "./provider-sync.service";

@Controller()
export class ProviderSyncController {
  constructor(private readonly providerSyncService: ProviderSyncService) {}

  @Get("connectors")
  async listConnectors() {
    return { items: await this.providerSyncService.listConnectors() };
  }

  @Get("connections")
  async listConnections(
    @Query("userId") userId?: string,
    @Query("connectionId") connectionId?: string,
  ) {
    if (!userId) {
      throw new BadRequestException("userId is required");
    }

    return {
      items: await this.providerSyncService.listConnections(userId, connectionId),
    };
  }

  @Get("accounts")
  async listAccounts(
    @Query("userId") userId?: string,
    @Query("connectionId") connectionId?: string,
  ) {
    if (!userId) {
      throw new BadRequestException("userId is required");
    }

    return {
      items: await this.providerSyncService.listAccounts(userId, connectionId),
    };
  }

  @Get("transactions")
  async listTransactions(
    @Query("userId") userId?: string,
    @Query("connectionId") connectionId?: string,
    @Query("accountId") accountId?: string,
    @Query("cursor") cursor?: string,
    @Query("fromDate") fromDate?: string,
    @Query("toDate") toDate?: string,
  ) {
    if (!userId || !connectionId || !accountId) {
      throw new BadRequestException("userId, connectionId, and accountId are required");
    }

    return this.providerSyncService.listTransactions(userId, connectionId, accountId, cursor, {
      fromDate,
      toDate,
    });
  }

  @Get("holdings")
  async listHoldings(
    @Query("userId") userId?: string,
    @Query("connectionId") connectionId?: string,
    @Query("accountId") accountId?: string,
  ) {
    if (!userId || !connectionId || !accountId) {
      throw new BadRequestException("userId, connectionId, and accountId are required");
    }

    return this.providerSyncService.listHoldings(userId, connectionId, accountId);
  }

  @Post("sync")
  async triggerSync(@Body() request: SyncRequestDto) {
    if (!request.userId) {
      throw new BadRequestException("userId is required");
    }

    return this.providerSyncService.triggerSync(request);
  }

  @Get("connect-url")
  async getConnectUrl(
    @Query("connectorId") connectorId?: string,
    @Query("redirectUri") redirectUri?: string,
  ) {
    return this.providerSyncService.getConnectUrl(connectorId, redirectUri);
  }

  @Post("connections/delete")
  async deleteConnection(
    @Query("userId") userId?: string,
    @Query("connectionId") connectionId?: string,
  ) {
    if (!userId || !connectionId) {
      throw new BadRequestException("userId and connectionId are required");
    }

    await this.providerSyncService.deleteConnection(userId, connectionId);
    return { success: true };
  }

  @Post("accounts/disable")
  async disableAccount(@Query("userId") userId?: string, @Query("accountId") accountId?: string) {
    if (!userId || !accountId) {
      throw new BadRequestException("userId and accountId are required");
    }

    await this.providerSyncService.disableAccount(userId, accountId);
    return { success: true };
  }

  @Get("sync-runs/:runId")
  async getSyncRun(@Param("runId") runId: string) {
    const run = await this.providerSyncService.getSyncRun(runId);
    if (!run) {
      throw new BadRequestException(`Sync run ${runId} not found`);
    }

    return run;
  }
}
