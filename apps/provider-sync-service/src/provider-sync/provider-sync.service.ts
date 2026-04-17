import { Injectable, InternalServerErrorException } from "@nestjs/common";

import type { SyncRequestDto } from "./dto";
import { ProvidersService } from "../providers/providers.service";
import { AppConfigService } from "../config/config.service";

@Injectable()
export class ProviderSyncService {
  constructor(
    private readonly providersService: ProvidersService,
    private readonly config: AppConfigService,
  ) {}

  async listConnectors() {
    return this.providersService.getProvider().listConnectors();
  }

  async listConnections(userId: string, connectionId?: string) {
    return this.providersService.getProvider().listConnections({ userId, connectionId });
  }

  async listAccounts(userId: string, connectionId?: string) {
    return this.providersService.getProvider().listAccounts({ userId, connectionId });
  }

  async listTransactions(
    userId: string,
    connectionId: string,
    accountId: string,
    cursor?: string,
    options?: { fromDate?: string; toDate?: string },
  ) {
    return this.providersService
      .getProvider()
      .listTransactions({
        userId,
        connectionId,
        accountId,
        cursor,
        fromDate: options?.fromDate,
        toDate: options?.toDate,
      });
  }

  async listHoldings(userId: string, connectionId: string, accountId: string) {
    return this.providersService.getProvider().listHoldings({ userId, connectionId, accountId });
  }

  async triggerSync(request: SyncRequestDto) {
    return this.providersService.getProvider().triggerSync({
      userId: request.userId,
      connectionId: request.connectionId,
      mode: request.mode,
      fromDate: request.fromDate,
      toDate: request.toDate,
    });
  }

  async getSyncRun(runId: string) {
    return this.providersService.getProvider().getSyncRun(runId);
  }

  async disableAccount(userId: string, accountId: string) {
    return this.providersService.getProvider().disableAccount({ userId, accountId });
  }

  async deleteConnection(userId: string, connectionId: string) {
    return this.providersService.getProvider().deleteConnection({ userId, connectionId });
  }

  async getConnectUrl(connectorId?: string, redirectUri?: string): Promise<{ url: string }> {
    const clientId = this.config.powensClientId;
    const clientSecret = this.config.powensClientSecret;
    const userId = this.config.powensUserId;

    if (!clientId || !clientSecret || !userId) {
      throw new InternalServerErrorException(
        "Powens credentials are required to generate a connect URL.",
      );
    }

    // Step 1: Get a user-scoped access token
    const renewResponse = await fetch(`${this.config.powensBaseUrl}/auth/renew`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        grant_type: "client_credentials",
        client_id: clientId,
        client_secret: clientSecret,
        id_user: Number(userId),
        revoke_previous: true,
      }),
    });

    if (!renewResponse.ok) {
      const text = await renewResponse.text();
      throw new InternalServerErrorException(
        `Failed to obtain Powens token: ${text.slice(0, 300)}`,
      );
    }

    const renewData = (await renewResponse.json()) as { access_token?: string };
    if (!renewData.access_token) {
      throw new InternalServerErrorException("Powens did not return an access_token.");
    }

    // Step 2: Exchange access token for a temporary code tied to this user
    const codeResponse = await fetch(`${this.config.powensBaseUrl}/auth/token/code`, {
      headers: { Authorization: `Bearer ${renewData.access_token}` },
    });

    if (!codeResponse.ok) {
      const text = await codeResponse.text();
      throw new InternalServerErrorException(
        `Failed to obtain Powens temporary code: ${text.slice(0, 300)}`,
      );
    }

    const codeData = (await codeResponse.json()) as { code?: string };
    if (!codeData.code) {
      throw new InternalServerErrorException("Powens did not return a temporary code.");
    }

    const finalRedirectUri =
      redirectUri ?? this.config.powensRedirectUri ?? "http://localhost:3001";
    let url = `${this.config.powensBaseUrl}/auth/webview/connect?client_id=${encodeURIComponent(clientId)}&redirect_uri=${encodeURIComponent(finalRedirectUri)}&code=${encodeURIComponent(codeData.code)}`;
    if (connectorId) {
      url += `&connector_uuids=${encodeURIComponent(connectorId)}`;
    }
    return { url };
  }
}
