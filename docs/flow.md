# Circulo — Flows (v1.0)

- **Status:** Draft v1.0 (2026-11-20)
- **Related:** [TRD](trd.md) · [UX](ux.md)

All flows use the neutral event protocol defined in the TRD.

## 1. App start

- Load persisted projects and sessions
- Start or attach provider drivers for managed projects
- Open SSE / channel subscription for live events

## 2. Add project

- User picks folder
- Core creates project record, optionally starts provider process or worktree
- Status events flow to UI (starting → running or error)

## 3. Send prompt (core loop)

- User sends prompt from composer
- Core creates or reuses session, calls provider driver
- Events stream: status busy → parts (text/reasoning/tool) → finish → status idle
- UI renders live via the same reducer used for history hydration

## 4. Steering & queue

- While a turn is running, additional messages can be queued or used to steer
- Driver translates to the provider’s native mechanism

## 5. Permission

- Provider emits permission request
- UI shows card above composer
- User chooses once / always / deny
- Response flows back through driver

## 6. Git checkpoint & rewind

- Before/after significant turns, core creates conversation-aware checkpoint
- User can rewind from UI → core restores Git state + conversation context

## 7. Open session / hydration

- Load history from local store or provider
- Live events upsert into the same model

## 8. Quit

- Graceful shutdown of all provider processes
- Persist any pending state