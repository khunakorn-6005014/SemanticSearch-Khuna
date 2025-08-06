## Backend Overview
The backend is a Rust/Axum service that manages an append-only vector index () and metadata store (). It exposes:
• 	POST /reviews: Insert a single review
• 	POST /reviews/bulk: Insert multiple reviews in one call
• 	POST /search: Semantic search by query, returns top-K matches
• 	GET /health: Health check

- Handlers live in src/handlers/reviews.rs.
- main.rs only wires routes and extensions—no business logic.
- Shared state (Store, Embedder) is injected via Extension.
- Error types live in src/error.rs, so handlers can return Result<…, ApiError>.
- store.rs only needs a helper next_id(); everything else (append/search) stays there.

src/
├── main.rs
├── error.rs
├── handlers/
│   └── reviews.rs
└── store.rs

## TEST
 -POST localhost:8080/reviews
 -POST localhost:8080/search

 