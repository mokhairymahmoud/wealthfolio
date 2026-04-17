import { Injectable } from "@nestjs/common";
import { resolve } from "node:path";

@Injectable()
export class AppConfigService {
  get host(): string {
    return process.env.HOST?.trim() || "127.0.0.1";
  }

  get port(): number {
    return Number(process.env.PORT ?? "3001");
  }

  get serviceToken(): string {
    return process.env.SERVICE_TOKEN?.trim() || "dev-token";
  }

  get provider(): "fixtures" | "powens" {
    const value = process.env.AGGREGATION_PROVIDER?.trim().toLowerCase();
    return value === "powens" ? "powens" : "fixtures";
  }

  get powensBaseUrl(): string {
    return process.env.POWENS_BASE_URL?.trim().replace(/\/$/, "") || "https://woob.biapi.pro/2.0";
  }

  get powensClientId(): string | null {
    return process.env.POWENS_CLIENT_ID?.trim() || null;
  }

  get powensClientSecret(): string | null {
    return process.env.POWENS_CLIENT_SECRET?.trim() || null;
  }

  get powensUserId(): string | null {
    return process.env.POWENS_USER_ID?.trim() || null;
  }

  get powensUserAccessToken(): string | null {
    return process.env.POWENS_USER_ACCESS_TOKEN?.trim() || null;
  }

  get powensCountryCodes(): string | null {
    return process.env.POWENS_COUNTRY_CODES?.trim() || null;
  }

  get powensRedirectUri(): string | null {
    return process.env.POWENS_REDIRECT_URI?.trim() || null;
  }

  get frontendUrl(): string | null {
    return process.env.FRONTEND_URL?.trim() || null;
  }

  get fixtureDataFile(): string {
    const configured = process.env.FIXTURE_DATA_FILE?.trim();
    if (configured) {
      return resolve(process.cwd(), configured);
    }

    return resolve(process.cwd(), "src/fixtures/sample-provider-data.json");
  }
}
