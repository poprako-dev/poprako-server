type JsonRecord = Record<string, unknown>;

export interface ApiResponse<T> {
  status: number;
  headers: Headers;
  body: ApiBody<T>;
}

export interface SuccessBody<T> {
  code: 0;
  data: T;
}

export interface ErrorBody {
  code: number;
  message?: string;
}

export type ApiBody<T> = T | string | null;

export class ApiClient {
  private token: string | null = null;

  constructor(private readonly baseUrl: string) {}

  setToken(token: string): void {
    this.token = token;
  }

  async get<T>(path: string): Promise<ApiResponse<T>> {
    return this.request<T>("GET", path);
  }

  async post<T>(path: string, body?: JsonRecord): Promise<ApiResponse<T>> {
    return this.request<T>("POST", path, body);
  }

  async put<T>(path: string, body?: JsonRecord): Promise<ApiResponse<T>> {
    return this.request<T>("PUT", path, body);
  }

  async patch<T>(path: string, body?: JsonRecord): Promise<ApiResponse<T>> {
    return this.request<T>("PATCH", path, body);
  }

  async delete<T>(path: string): Promise<ApiResponse<T>> {
    return this.request<T>("DELETE", path);
  }

  private async request<T>(
    method: string,
    path: string,
    body?: JsonRecord,
  ): Promise<ApiResponse<T>> {
    const headers = new Headers();

    if (body !== undefined) {
      headers.set("content-type", "application/json");
    }

    if (this.token) {
      headers.set("authorization", `Bearer ${this.token}`);
    }

    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });

    const text = await response.text();
    const parsedBody = parseBody<T>(text);

    return {
      status: response.status,
      headers: response.headers,
      body: parsedBody,
    };
  }
}

function parseBody<T>(text: string): ApiBody<T> {
  if (text.length === 0) {
    return null;
  }

  try {
    return JSON.parse(text) as T;
  } catch {
    return text;
  }
}
