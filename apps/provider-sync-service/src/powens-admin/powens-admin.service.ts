import {
  Injectable,
  InternalServerErrorException,
} from "@nestjs/common";

import { AppConfigService } from "../config/config.service";
import type { PowensInitTokenRequestDto, PowensRenewTokenRequestDto } from "./powens-admin.dto";

interface PowensAuthInitResponse {
  auth_token?: string;
  type?: string;
  id_user?: number;
  expires_in?: number | null;
}

interface PowensAuthRenewResponse {
  access_token?: string;
  token_type?: string;
  expires_in?: number | null;
}

@Injectable()
export class PowensAdminService {
  constructor(private readonly config: AppConfigService) {}

  async initUserToken(
    request: PowensInitTokenRequestDto = {},
  ): Promise<PowensAuthInitResponse> {
    const clientId = request.clientId ?? this.config.powensClientId ?? undefined;
    const clientSecret =
      request.clientSecret ?? this.config.powensClientSecret ?? undefined;

    const body: Record<string, string> = {};
    if (clientId) {
      body.client_id = clientId;
    }
    if (clientSecret) {
      body.client_secret = clientSecret;
    }

    return this.request<PowensAuthInitResponse>("/auth/init", body);
  }

  async renewUserToken(
    request: PowensRenewTokenRequestDto = {},
  ): Promise<PowensAuthRenewResponse> {
    const clientId = request.clientId ?? this.config.powensClientId;
    const clientSecret = request.clientSecret ?? this.config.powensClientSecret;

    if (!clientId || !clientSecret) {
      throw new InternalServerErrorException(
        "Powens client credentials are required for auth/renew.",
      );
    }

    const resolvedUserId = request.userId ?? this.config.powensUserId ?? undefined;
    const numericUserId =
      resolvedUserId === undefined || resolvedUserId === null || resolvedUserId === ""
        ? undefined
        : Number(resolvedUserId);

    if (resolvedUserId !== undefined && Number.isNaN(numericUserId)) {
      throw new InternalServerErrorException(
        "Powens auth/renew requires a numeric userId when provided.",
      );
    }

    const body: Record<string, string | number | boolean> = {
      grant_type: "client_credentials",
      client_id: clientId,
      client_secret: clientSecret,
      revoke_previous: request.revokePrevious ?? false,
    };

    if (numericUserId !== undefined) {
      body.id_user = numericUserId;
    }

    return this.request<PowensAuthRenewResponse>("/auth/renew", body);
  }

  private async request<T>(pathname: string, body: unknown): Promise<T> {
    const response = await fetch(`${this.config.powensBaseUrl}${pathname}`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(body),
    });

    const text = await response.text();
    if (!response.ok) {
      throw new InternalServerErrorException(
        `Powens request failed (${response.status}): ${text}`,
      );
    }

    return JSON.parse(text) as T;
  }
}
