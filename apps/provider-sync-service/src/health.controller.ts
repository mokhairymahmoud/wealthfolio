import { Controller, Get, Query, Res } from "@nestjs/common";
import type { Response } from "express";

import { Public } from "./auth/public.decorator";
import { AppConfigService } from "./config/config.service";

@Controller()
export class HealthController {
  @Public()
  @Get("healthz")
  healthz() {
    return { ok: true };
  }
}

@Controller()
export class CallbackController {
  constructor(private readonly config: AppConfigService) {}

  @Public()
  @Get("/")
  callback(@Query("connection_id") connectionId: string | undefined, @Res() res: Response) {
    const frontendUrl = this.config.frontendUrl;
    if (frontendUrl) {
      const target = new URL("/provider/callback", frontendUrl);
      if (connectionId) {
        target.searchParams.set("connection_id", connectionId);
      }
      return res.redirect(302, target.toString());
    }

    const message = connectionId
      ? `Bank connected successfully (connection ${connectionId}).`
      : "Connection complete.";
    return res.setHeader("Content-Type", "text/html").send(`<!DOCTYPE html>
<html><head><title>Wealthfolio</title>
<style>body{font-family:system-ui,sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;margin:0;background:#fafafa}
.card{background:#fff;border-radius:12px;padding:2rem 3rem;box-shadow:0 2px 8px rgba(0,0,0,.08);text-align:center}
h1{font-size:1.25rem;margin:0 0 .5rem}p{color:#666;margin:0}</style></head>
<body><div class="card"><h1>${message}</h1><p>You can close this tab and return to Wealthfolio.</p></div></body></html>`);
  }
}
