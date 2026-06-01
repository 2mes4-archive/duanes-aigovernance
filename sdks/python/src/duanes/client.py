from __future__ import annotations

import json
from typing import Any, Optional

import httpx
from pydantic import BaseModel


class ChatMessage(BaseModel):
    role: str
    content: str
    name: Optional[str] = None


class InferenceRequest(BaseModel):
    model: str
    messages: list[ChatMessage]
    temperature: Optional[float] = None
    top_p: Optional[float] = None
    max_tokens: Optional[int] = None
    stream: Optional[bool] = None
    stop: Optional[list[str]] = None
    user: Optional[str] = None


class TokenUsage(BaseModel):
    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0


class Choice(BaseModel):
    index: int
    message: ChatMessage
    finish_reason: Optional[str] = None


class InferenceResponse(BaseModel):
    id: str
    object: str
    created: int
    model: str
    choices: list[Choice]
    usage: TokenUsage


class DuanesClient:
    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        user_id: Optional[str] = None,
        app_id: Optional[str] = None,
        team_id: Optional[str] = None,
        org_id: Optional[str] = None,
        api_key: Optional[str] = None,
        timeout: float = 120.0,
    ):
        self._base_url = base_url.rstrip("/")
        self._user_id = user_id
        self._app_id = app_id
        self._team_id = team_id
        self._org_id = org_id
        self._timeout = timeout

        headers: dict[str, str] = {"Content-Type": "application/json"}
        if api_key:
            headers["Authorization"] = f"Bearer {api_key}"

        self._client = httpx.Client(
            base_url=self._base_url,
            headers=headers,
            timeout=timeout,
        )

    def _inject_identity(self, headers: dict[str, str]) -> None:
        if self._user_id:
            headers["X-Duanes-User-Id"] = self._user_id
        if self._app_id:
            headers["X-Duanes-App-Id"] = self._app_id
        if self._team_id:
            headers["X-Duanes-Team-Id"] = self._team_id
        if self._org_id:
            headers["X-Duanes-Org-Id"] = self._org_id

    def chat_completions(
        self,
        model: str,
        messages: list[ChatMessage],
        temperature: Optional[float] = None,
        max_tokens: Optional[int] = None,
        stream: Optional[bool] = None,
        user: Optional[str] = None,
    ) -> InferenceResponse:
        request = InferenceRequest(
            model=model,
            messages=messages,
            temperature=temperature,
            max_tokens=max_tokens,
            stream=stream,
            user=user,
        )

        headers: dict[str, str] = {}
        self._inject_identity(headers)

        response = self._client.post(
            "/v1/chat/completions",
            json=request.model_dump(exclude_none=True),
            headers=headers,
        )
        response.raise_for_status()

        return InferenceResponse(**response.json())

    def list_models(self) -> list[dict[str, Any]]:
        response = self._client.get("/v1/models")
        response.raise_for_status()
        return response.json().get("data", [])

    def daily_stats(self) -> dict[str, Any]:
        headers: dict[str, str] = {}
        self._inject_identity(headers)

        response = self._client.get("/api/v1/stats/daily", headers=headers)
        response.raise_for_status()
        return response.json()

    def recent_records(self, limit: int = 50) -> list[dict[str, Any]]:
        headers: dict[str, str] = {}
        self._inject_identity(headers)

        response = self._client.get("/api/v1/stats/recent", headers=headers)
        response.raise_for_status()
        return response.json()

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> DuanesClient:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()
