export const API_ENDPOINT = "/v2/orders";

export function buildOrdersUrl(orderId: string): string {
  return `${API_ENDPOINT}/${orderId}`;
}

export function getOrdersRouteLabel(): string {
  return "Orders";
}
