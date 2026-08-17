# AI Assistant Architecture

This document describes the technical architecture of LibreTune's bring-your-own-LLM
AI Assistant. It is intended for developers contributing to or extending the feature.

## Design Principles

1. **Bring your own model** — LibreTune never hosts a model. The user supplies the
   provider, key, and model. This keeps the feature dependency-free and private.
2. **Propose, never apply** — the assistant's only output is a validated, clamped
   *proposal*. Application and burn are always separate, user-triggered steps.
3. **Reuse existing primitives** — the assistant is a thin layer over the existing
   `Action` enum, `validate_action_set`, AutoTune engines, and INI metadata. It does
   not duplicate safety logic.
4. **Provider-agnostic** — a single `Provider` trait abstracts all LLM backends. The
   orchestrator never sees provider-specific JSON.

## Module Layout

```
crates/libretune-core/src/
├── agent/
│   ├── mod.rs           # module root + re-exports
│   ├── orchestrator.rs  # multi-turn agent loop + ReadToolExecutor trait
│   ├── tools.rs         # tool catalogue (JSON-schema tool definitions)
│   ├── context.rs       # context-gathering helpers (constants, tables)
│   ├── summarize.rs     # summarize_tune_context() aggregation
│   ├── safety.rs        # authority-limit clamping
│   └── tiers.rs         # constant safety tiering (Safe/Caution/Dangerous)
└── llm/
    ├── mod.rs           # module root
    ├── types.rs         # ChatRequest/ChatResponse/Message/ToolCall/LlmError
    ├── provider.rs      # Provider trait + factory
    ├── client.rs        # LlmClient (top-level entry point)
    └── providers/
        ├── openai.rs    # OpenAI Chat Completions (native protocol)
        ├── anthropic.rs # Anthropic Messages API (native protocol)
        └── google.rs    # Google Gemini generateContent (native protocol)
```

The Tauri layer wraps these in `crates/libretune-app/src-tauri/src/commands/agent.rs`
(commands: `agent_status`, `agent_send_message`, `agent_apply_proposals`).

## The Agent Loop

One user turn runs a **multi-turn loop** inside `orchestrator::run_turn`:

```
user message
     │
     ▼
┌─────────────────────────────────────────────┐
│  build ChatRequest (system + history + msg) │
│  + tool catalogue                            │
└──────────────────────┬──────────────────────┘
                       ▼
              call Provider::chat
                       │
        ┌──────────────┴───────────────┐
        ▼                              ▼
  read tool calls?              propose tool calls?
        │                              │
        ▼                              ▼
  execute via                 map → Action[]
  ReadToolExecutor            validate_action_set
        │                     clamp to authority
        ▼                     accumulate into Proposal
  append tool-result                   │
  messages to history                  │
        │                              │
        ▼                              ▼
  loop back ─────────────►  (no reads left? → done)
                                   │
                                   ▼
                            return Proposal
```

The loop is bounded by `MAX_READ_ROUNDS` (6) to cap cost and prevent runaway
conversations. Read results are fed back as tool-result messages so the model can
reason over actual table/constant data before emitting its final reply.

## The Provider Trait

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError>;
}
```

Each concrete provider translates the generic `ChatRequest` to its wire format
(OpenAI `tools[]`, Anthropic `tool_use` blocks, Gemini `functionDeclarations`),
calls its endpoint via the shared `reqwest` client, and parses the response back
into a generic `ChatResponse`. Adding a new provider requires only implementing
this trait — no orchestrator changes.

## The ReadToolExecutor Trait

```rust
#[async_trait]
pub trait ReadToolExecutor: Send + Sync {
    fn handles(&self, tool_name: &str) -> bool;
    async fn execute(&self, tool_name: &str, arguments: &str) -> String;
}
```

The core library defines this contract; the Tauri layer implements it as
`LiveReadExecutor`, which reaches `AppState` via `tauri::Manager::state()` and
reads tables/constants against the loaded definition and tune. This split keeps
the loop unit-testable without a live provider or ECU.

## Tool Catalogue

Defined in `agent/tools.rs`. The model may call:

| Tool | Type | Purpose |
|------|------|---------|
| `list_tables` | read | Discover table names + roles + dimensions |
| `read_table` | read | Get a table's values, axis bins, units |
| `read_constant` | read | Get a constant's value, min/max, options |
| `list_features` | read | List feature-toggle (bits) constants |
| `summarize_tune_context` | read | Aggregated coverage + AFR error + anomalies |
| `tune_health_check` | read | Per-region health scores |
| `propose_table_edit` | propose | Stage a single-cell edit (reviewed + clamped) |
| `propose_bulk_operation` | propose | Stage scale/smooth/interpolate (reviewed) |
| `propose_constant_change` | propose | Stage a constant change (tier-flagged) |

## TableRole Inference

`EcuDefinition::infer_table_roles()` attaches a machine-readable `TableRole`
enum (`Ve`, `Ignition`, `AfrTarget`, `WarmupEnrichment`, `Other`) to every
`TableDefinition`, derived from the INI's `[VeAnalyze]` and `[WueAnalyze]`
sections. This lets the assistant know what a table *does* without guessing from
its name.

## Validation Extensions

The assistant motivated extending `ActionPlayer::validate_action_set` beyond
existence checks. It now validates:

- **Constant `min`/`max`** (display-unit bounds)
- **`DataType` raw storage range** (pre-scale)
- **Table cell-index bounds** (`x_index`/`y_index` vs `x_size`/`y_size`)
- **Bits-type enumeration** (value must be a valid option index)

## Frontend

- `components/agent/AgentSidePanel.tsx` — the docked right-hand panel (header,
  resize handle, pop-out/collapse buttons, collapsible review queue).
- `components/agent/ChatPanel.tsx` — the conversational transcript + input.
- `components/agent/ProposalQueue.tsx` — the per-item review surface.
- `components/common/RiskAcknowledgement.tsx` — reusable risk-ack primitive.
- The panel can pop out via the existing `WebviewWindow` + hash-routing system
  (see `PopOutWindow.tsx`, type `agent`).

## Error Handling

- The core layer uses a typed `LlmError` enum (`Network`, `Auth`, `RateLimit`,
  `Parse`, `ApiError`, `Config`).
- Tauri commands flatten to `Result<T, String>` at the boundary.
- Settings saves are per-setting (one failure does not abort the others).
