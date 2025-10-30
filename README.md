# SWH Graph Metadata

This project rely on the Software Heritage Rust API to compute repository level metadata exposed by a REST API.


- **Latest Commit Dates**: Identifies the most recent commit timestamp for each origin
- **Commit Counts**: Computes total number of commits in the latest snapshot
- **Committer Statistics**: Analyzes unique contributor counts per origin
- **Snapshot Analysis**: Filters origins based on snapshot availability



```
src/
├── server.rs         # REST API server implementation
├── graph.rs          # Core graph processing and caching logic
├── origin.rs         # Origin data structures and computation methods
└── utils.rs          # Utility functions for graph operations
```

## Usage

### Launch REST server
```
cargo run --bin swh-server -- --graph-path "graph_path"
```

### Available API Endpoints

#### Bulk Data Retrieval
- `GET /origins` - List all origin IDs (filtered for valid origins)
- `GET /origins/latest-commit-dates` - All origins' latest commit dates
- `GET /origins/commit-counts` - All origins' commit counts  
- `GET /origins/committer-counts` - All origins' committer counts

#### Individual Origin Queries
- `GET /origins/:id/url` - Specific origin URL
- `GET /origins/:id/latest-commit-date` - Specific origin latest commit
- `GET /origins/:id/committer-count` - Specific origin committer count
- `GET /origins/:id/commit-count` - Specific origin commit count