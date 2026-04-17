export interface PowensInitTokenRequestDto {
  clientId?: string;
  clientSecret?: string;
}

export interface PowensRenewTokenRequestDto {
  clientId?: string;
  clientSecret?: string;
  userId?: string | number;
  revokePrevious?: boolean;
}
