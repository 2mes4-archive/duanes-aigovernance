package duanes

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

type Client struct {
	baseURL    string
	httpClient *http.Client
	headers    map[string]string
}

type ClientOption func(*Client)

func WithUserID(userID string) ClientOption {
	return func(c *Client) {
		c.headers["X-Duanes-User-Id"] = userID
	}
}

func WithAppID(appID string) ClientOption {
	return func(c *Client) {
		c.headers["X-Duanes-App-Id"] = appID
	}
}

func WithTeamID(teamID string) ClientOption {
	return func(c *Client) {
		c.headers["X-Duanes-Team-Id"] = teamID
	}
}

func WithOrgID(orgID string) ClientOption {
	return func(c *Client) {
		c.headers["X-Duanes-Org-Id"] = orgID
	}
}

func WithAPIKey(apiKey string) ClientOption {
	return func(c *Client) {
		c.headers["Authorization"] = "Bearer " + apiKey
	}
}

func WithTimeout(timeout time.Duration) ClientOption {
	return func(c *Client) {
		c.httpClient.Timeout = timeout
	}
}

func NewClient(baseURL string, opts ...ClientOption) *Client {
	c := &Client{
		baseURL: baseURL,
		httpClient: &http.Client{
			Timeout: 120 * time.Second,
		},
		headers: map[string]string{
			"Content-Type": "application/json",
		},
	}

	for _, opt := range opts {
		opt(c)
	}

	return c
}

type ChatMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
	Name    string `json:"name,omitempty"`
}

type ChatCompletionRequest struct {
	Model       string        `json:"model"`
	Messages    []ChatMessage `json:"messages"`
	Temperature *float64      `json:"temperature,omitempty"`
	TopP        *float64      `json:"top_p,omitempty"`
	MaxTokens   *int64        `json:"max_tokens,omitempty"`
	Stream      *bool         `json:"stream,omitempty"`
	Stop        []string      `json:"stop,omitempty"`
	User        string        `json:"user,omitempty"`
}

type TokenUsage struct {
	PromptTokens     int64 `json:"prompt_tokens"`
	CompletionTokens int64 `json:"completion_tokens"`
	TotalTokens      int64 `json:"total_tokens"`
}

type Choice struct {
	Index        int          `json:"index"`
	Message      ChatMessage  `json:"message"`
	FinishReason *string      `json:"finish_reason"`
}

type ChatCompletionResponse struct {
	ID      string     `json:"id"`
	Object  string     `json:"object"`
	Created int64      `json:"created"`
	Model   string     `json:"model"`
	Choices []Choice   `json:"choices"`
	Usage   TokenUsage `json:"usage"`
}

type DailyStats struct {
	TotalRequests int64   `json:"total_requests"`
	TotalTokens   int64   `json:"total_tokens"`
	TotalCost     float64 `json:"total_cost"`
	AvgLatencyMs  float64 `json:"avg_latency_ms"`
}

func (c *Client) ChatCompletions(ctx context.Context, req ChatCompletionRequest) (*ChatCompletionResponse, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("marshal request: %w", err)
	}

	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, c.baseURL+"/v1/chat/completions", bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}

	for k, v := range c.headers {
		httpReq.Header.Set(k, v)
	}

	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("http request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		respBody, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("api error %d: %s", resp.StatusCode, string(respBody))
	}

	var result ChatCompletionResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("decode response: %w", err)
	}

	return &result, nil
}

func (c *Client) DailyStats(ctx context.Context) (*DailyStats, error) {
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+"/api/v1/stats/daily", nil)
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}

	for k, v := range c.headers {
		httpReq.Header.Set(k, v)
	}

	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("http request: %w", err)
	}
	defer resp.Body.Close()

	var result DailyStats
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("decode response: %w", err)
	}

	return &result, nil
}

func (c *Client) Health(ctx context.Context) (string, error) {
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+"/health", nil)
	if err != nil {
		return "", fmt.Errorf("create request: %w", err)
	}

	resp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return "", fmt.Errorf("http request: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("read response: %w", err)
	}

	return string(body), nil
}
