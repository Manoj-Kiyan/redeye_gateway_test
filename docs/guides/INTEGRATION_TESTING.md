# Integration Testing

## Goal

Validate the local RedEye stack end to end after starting the services with `.\dev.bat`.

## Covered Flow

The smoke test verifies:

1. auth health
2. gateway health and readiness
3. signup
4. onboarding
5. provider credential update
6. tenant route save
7. route dry-run
8. audit trail read
9. gateway metrics read

## Run

From the repository root:

```powershell
.\scripts\integration-smoke.ps1
```

## Notes

- The script creates a unique tenant email and workspace each run.
- It uses a dummy OpenAI key for integration coverage only.
- It does not attempt a real upstream LLM completion.
- Services must already be running locally.

## Expected Success Output

You should see:

- `Integration smoke test passed.`
- generated tenant email
- workspace name
- gateway readiness status
- resolved provider from route dry-run

## When To Use

- after major auth or gateway changes
- before pushing large refactors
- before demoing the local platform
- when onboarding a new teammate
