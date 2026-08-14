@echo off
set ANTHROPIC_BASE_URL=http://127.0.0.1:1234
set ANTHROPIC_API_KEY=sk-ant-your-key-here
set ANTHROPIC_MODEL=WhitchSupportImageReady
set CLAW_WORKSPACE_POLICY=allow
set CLAW_BIN=rust\target\release\claw.exe
"%CLAW_BIN%" %*
pause