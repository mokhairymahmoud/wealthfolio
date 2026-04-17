import {
  CanActivate,
  ExecutionContext,
  Injectable,
  UnauthorizedException,
} from "@nestjs/common";
import { Reflector } from "@nestjs/core";

import { AppConfigService } from "../config/config.service";
import { IS_PUBLIC_KEY } from "./public.decorator";

@Injectable()
export class ServiceTokenGuard implements CanActivate {
  constructor(
    private readonly config: AppConfigService,
    private readonly reflector: Reflector,
  ) {}

  canActivate(context: ExecutionContext): boolean {
    const isPublic = this.reflector.getAllAndOverride<boolean>(IS_PUBLIC_KEY, [
      context.getHandler(),
      context.getClass(),
    ]);

    if (isPublic) {
      return true;
    }

    const request = context.switchToHttp().getRequest<{ headers: Record<string, string | string[]> }>();
    const rawAuth = request.headers.authorization;
    const authorization = Array.isArray(rawAuth) ? rawAuth[0] : rawAuth;

    if (!authorization?.startsWith("Bearer ")) {
      throw new UnauthorizedException("Missing bearer token");
    }

    const token = authorization.slice("Bearer ".length).trim();
    if (token !== this.config.serviceToken) {
      throw new UnauthorizedException("Invalid bearer token");
    }

    return true;
  }
}
