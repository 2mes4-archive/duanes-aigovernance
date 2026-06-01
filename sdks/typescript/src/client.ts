interface DuanesClientConfig {
  baseUrl?: string;
  userId?: string;
  appId?: string;
  teamId?: string;
  orgId?: string;
  apiKey?: string;
  timeout?: number;
}

interface ChatMessage {
  role: "system" | "user" | "assistant" | "tool";
  content: string;
  name?: string;
}

interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  top_p?: number;
  max_tokens?: number;
  stream?: boolean;
  stop?: string[];
  user?: string;
}

interface TokenUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

interface Choice {
  index: number;
  message: ChatMessage;
  finish_reason: string | null;
}

interface ChatCompletionResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: Choice[];
  usage: TokenUsage;
}

interface DailyStats {
  total_requests: number;
  total_tokens: number;
  total_cost: number;
  avg_latency_ms: number;
}

export class DuanesClient {
  private baseUrl: string;
  private headers: Record<string, string>;
  private timeout: number;

  constructor(config: DuanesClientConfig = {}) {
    this.baseUrl = (config.baseUrl || "http://localhost:8080").replace(/\/$/, "");
    this.timeout = config.timeout || 120000;
    this.headers = {
      "Content-Type": "application/json",
    };

    if (config.apiKey) {
      this.headers["Authorization"] = `Bearer ${config.apiKey}`;
    }
    if (config.userId) {
      this.headers["X-Duanes-User-Id"] = config.userId;
    }
    if (config.appId) {
      this.headers["X-Duanes-App-Id"] = config.appId;
    }
    if (config.teamId) {
      this.headers["X-Duanes-Team-Id"] = config.teamId;
    }
    if (config.orgId) {
      this.headers["X-Duanes-Org-Id"] = config.orgId;
    }
  }

  async chatCompletions(
    request: ChatCompletionRequest
  ): Promise<ChatCompletionResponse> {
    const response = await fetch(`${this.baseUrl}/v1/chat/completions`, {
      method: "POST",
      headers: this.headers,
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Duanes API error: ${response.status} ${await response.text()}`);
    }

    return response.json();
  }

  async listModels(): Promise<{ object: string; data: Array<{ id: string; object: string; owned_by: string }> }> {
    const response = await fetch(`${this.baseUrl}/v1/models`, {
      headers: this.headers,
    });

    if (!response.ok) {
      throw new Error(`Duanes API error: ${response.status}`);
    }

    return response.json();
  }

  async dailyStats(): Promise<DailyStats> {
    const response = await fetch(`${this.baseUrl}/api/v1/stats/daily`, {
      headers: this.headers,
    });

    if (!response.ok) {
      throw new Error(`Duanes API error: ${response.status}`);
    }

    return response.json();
  }

  async recentRecords(limit?: number): Promise<unknown[]> {
    const response = await fetch(`${this.baseUrl}/api/v1/stats/recent`, {
      headers: this.headers,
    });

    if (!response.ok) {
      throw new Error(`Duanes API error: ${response.status}`);
    }

    const data = await response.json();
    return Array.isArray(data) ? data : [];
  }

  async health(): Promise<string> {
    const response = await fetch(`${this.baseUrl}/health`);
    return response.text();
  }
}
