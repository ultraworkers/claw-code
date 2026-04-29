"""Mock-mode runner: returns canned responses with realistic timing.

Used when CLAW_WEB_MODE=mock (the default), so the UI walks end-to-end on
a fresh checkout without needing claw plumbed. Keeps the same TurnEvent
shape as `ClawRunner` so the WebSocket handler doesn't branch.

Includes a small "scripted approval" affordance: prompts that begin with
`@@approve` trigger one fake approval_request before producing the canned
response. Lets us exercise the modal flow in mock mode.
"""

from __future__ import annotations

import asyncio
import random
import uuid
from pathlib import Path
from typing import AsyncIterator

from .claw_runner import ApprovalCallback, ApprovalRequest, TurnEvent


_CANNED_REPLIES = [
    "Looks like a Rust workspace. Want me to run `cargo test --workspace`?",
    "I read the file. The function at L42 looks suspicious — `unwrap()` "
    "on a value that can be `None` when the input is empty.",
    "Done. Created the file and added it to `mod.rs`.",
    "I traced the bug to a missing `await` on the spawn call. Fix below.",
    "Could you clarify whether you want this as a new crate or as a module "
    "inside the existing `runtime` crate?",
]


class MockRunner:
    def __init__(self, *_args, **_kwargs):
        # Stable mock session id per process; lets the UI exercise the
        # `--resume` flow visually even though nothing's actually persisted.
        self._session_id = f"mock-{uuid.uuid4().hex[:8]}"

    def is_available(self) -> bool:
        return True

    async def run_turn(
        self,
        cwd: Path,
        prompt: str,
        claw_session_id: str | None,
        env_overrides: dict[str, str] | None = None,
        approval_callback: ApprovalCallback | None = None,
    ) -> AsyncIterator[TurnEvent]:
        yield TurnEvent(
            "status",
            {"message": "starting", "mock": True, "cwd": str(cwd)},
        )
        await asyncio.sleep(0.2)
        yield TurnEvent("status", {"message": "running", "mock": True})

        # Optional scripted approval flow for exercising the UI modal.
        approved: bool | None = None
        if prompt.lstrip().startswith("@@approve") and approval_callback is not None:
            req = ApprovalRequest(
                tool="bash",
                current_mode="read-only",
                required_mode="workspace-write",
                reason="(mock) demonstrating approval flow",
                input='{"command":"ls -la","description":"list files"}',
            )
            yield TurnEvent("approval_request", req.as_dict())
            approved = bool(await approval_callback(req))

        await asyncio.sleep(0.4 + random.random() * 0.3)
        text = f"(mock) You said: “{prompt}”\n\n{random.choice(_CANNED_REPLIES)}"
        if approved is True:
            text += "\n\n[mock] approval granted; would have run the tool."
        elif approved is False:
            text += "\n\n[mock] approval denied; tool skipped."
        yield TurnEvent(
            "final",
            {
                "text": text,
                "claw_session_id": claw_session_id or self._session_id,
                "raw": False,
                "mock": True,
                "approvals_handled": 1 if approved is not None else 0,
            },
        )
