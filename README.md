# SWH Graph Analytics: A High-Performance Tool for Mining Software Heritage Data

## Abstract

This repository presents a high-performance Rust-based tool for analyzing Software Heritage (SWH) graph data, designed to efficiently process millions of software origins and their associated metadata. The tool provides both command-line batch processing capabilities and a REST API server for interactive analysis of large-scale software repository data.

## Overview

Software Heritage maintains the world's largest archive of software source code, containing billions of files and their complete development history. Analyzing this massive dataset requires specialized tools capable of handling the scale and complexity of the graph structure. This project implements a parallel processing framework for extracting and analyzing key metrics from SWH graph data.

## Key Features

### Graph Processing Engine
- **Parallel Origin Analysis**: Processes software origins using Rayon for multi-threaded computation
- **Memory-Efficient Caching**: Supports multiple serialization formats (JSON, Bincode) with intelligent caching
- **Scalable Architecture**: Handles datasets with millions of origins through chunked processing
- **Progress Monitoring**: Real-time progress bars with ETA and processing rate statistics

### Origin Metrics Extraction
- **Latest Commit Dates**: Identifies the most recent commit timestamp for each origin
- **Commit Counts**: Computes total number of commits in the latest snapshot
- **Committer Statistics**: Analyzes unique contributor counts per origin
- **Snapshot Analysis**: Filters origins based on snapshot availability

### REST API Server
- **Bulk Data Access**: Endpoints for retrieving all origins' metrics simultaneously
- **Individual Origin Queries**: Detailed information for specific origins
- **Debug Logging**: Comprehensive request/response logging for analysis
- **CORS Support**: Cross-origin resource sharing for web-based tools

### Performance Optimizations
- **Timeout Handling**: 1-second timeout mechanism to handle problematic origins
- **Error Recovery**: Robust error handling with detailed logging
- **Memory Management**: Efficient memory usage for large-scale processing
- **Checkpoint Saving**: Periodic saves during long-running computations

## Technical Architecture

### Core Components

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