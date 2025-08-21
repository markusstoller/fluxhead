<p align="center">
  <img src="fluxhead.png" alt="Fluxhead" width="100%" />
</p>

# Fluxhead

A Rust library and toolkit for parsing and inspecting Commodore 1541/CBM DOS GCR data from G64 disk images. It provides helpers to read G64 headers, track locations, and decode GCR-encoded track headers and data into their binary representation.

## Contents
- Overview
- Features
- Limitations
- Getting Started
  - Requirements
  - Installation
  - Build
  - Run tests
- Usage
  - Quick example
  - Working with tracks
- API Overview
- Project Structure
- Development
- Contributing
- License
- Acknowledgments

## Overview
Fluxhead focuses on reading and parsing G64 disk images:
- Parse G64 headers and track location tables
- Identify CBM DOS track headers and track data markers in GCR streams
- Decode GCR into raw bytes using cbm_dos::GCR
- Provide simple structs to represent parsed header/data pairs per track

This is useful for experimental disk analysis, visualization, or building higher-level tooling for Commodore 1541 disk formats.

## Features
- G64 header parsing (tag, revision, track count, track size)
- Track location decoding (full and half tracks)
- Track scanning for CBM DOS header and data patterns (0xFF sync and identifiers)
- GCR decoding via the cbm_dos crate
- Minimal, extensible Rust API aimed at experimentation

## Limitations
- Error handling is currently limited. The parser assumes relatively well-formed inputs and may return None or skip content when encountering unexpected structures. Comprehensive error reporting and recovery are planned but not yet implemented.
- Tracks without sync marks (no 0xFF sync bytes) are currently not handled. The parser relies on sync marks to locate CBM DOS headers and data; tracks lacking sync are not decoded at this time.

## Getting Started

### Requirements
- Rust (stable). Install via https://rustup.rs
- A G64 image file if you want to run the included parsing example/tests

### Installation
Add to your Cargo.toml if using as a dependency:

```toml
[dependencies]
fluxhead = { path = "." }
```

Or, if published on crates.io in the future:

```toml
[dependencies]
fluxhead = "0.x"
```

### Build
From the repository root:

```bash
cargo build
```

### Run tests
```bash
cargo test
```

## Usage

### Quick example
The library provides a simple Fluxhead type to parse in-memory buffers and print debug details:

```rust
use fluxhead::Fluxhead;

fn main() {
    // Load a G64 file into memory and parse it
    let data = std::fs::read("path/to/disk.g64").expect("read g64");
    let mut flux = Fluxhead::new();
    flux.parse_from_buffer(&data);
}
```

If you have a path on disk and want to use the convenience method in tests (as demonstrated in the repo tests):

```rust
use fluxhead::Fluxhead;

fn parse_file(path: &str) {
    let mut flux = Fluxhead::new();
    // Parses the file and returns an optional ParsedData
    let _parsed = flux.parse_from_file(path);
}
```

### Working with tracks
Internally, the parser scans for sync patterns (0xFF bytes) and known identifiers:
- 0x52 indicates a track header (CBM DOS header block)
- 0x55 indicates a track data block

When found, it decodes the GCR-encoded segments using cbm_dos::GCR and populates simple structures:

```rust
pub struct SectorHeader {
    pub gcr_encoded_data: Vec<u8>,
    pub gcr_decoded_data: Vec<u8>,
}

pub struct SectorData {
    pub gcr_encoded_data: Vec<u8>,
    pub gcr_decoded_data: Vec<u8>,
}

pub struct Sector {
    pub header: SectorHeader,
    pub data: SectorData,
}

pub struct Track {
    pub tracks: Option<Vec<Sector>>,
    pub speed_zone: i32,
}
```

You can adapt the parsing to collect all tracks and use them downstream (e.g., integrity checks, metadata extraction).

## API Overview
Key types (see src/lib.rs):
- G64HeaderInfo: basic G64 header
- TrackLocation: offsets for full and half tracks (internal)
- G64TrackData: per-track size metadata (internal)
- Fluxhead: orchestrates loading, parsing, and decoding (currently internal)

Public items exposed for consumers:
- G64HeaderInfo
- SectorHeader, SectorData, Sector, Track
- ParsedData

The implementation uses the endian_codec and cbm_dos crates for binary and GCR decoding.

## Project Structure
- src/lib.rs: core types and parsing logic
- fluxhead.png: project banner image (displayed above)
- Cargo.toml: crate metadata and dependencies

## Development
- Format: `cargo fmt`
- Lint: `cargo clippy`
- Test: `cargo test`
- Run examples or add your own under examples/

Tips:
- Use small G64 images for quick iteration
- Enable logging/printlns as needed when exploring new disks

## Contributing
Contributions are welcome! Please open an issue or pull request. When contributing code:
- Include tests when possible
- Keep changes minimal and focused
- Adhere to Rust style guidelines

## License
Specify your preferred license here (e.g., MIT/Apache-2.0). Add the corresponding LICENSE files to the repository.

## Acknowledgments
- cbm_dos crate for GCR encoding/decoding utilities
- Commodore community documentation around G64 and CBM DOS formats
