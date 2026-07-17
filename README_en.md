# Tarantula

A high-throughput, low-latency Reverse Geocoding server designed to handle massive traffic: practical usage of public open data and a modern technical stack.

## Abstract
In modern mobility and location-based services, displaying the user's current address based on their coordinates is a fundamental user experience (UX). 

The hierarchical boundary polygons that make up these addresses are public open data maintained by national governments. However, finding a free, production-ready public API capable of handling high-volume reverse geocoding requests is extremely difficult. Consequently, companies often have to rely on expensive paid APIs from major location service providers such as Google, Naver, or Kakao.

This project introduces a robust, highly optimized, and cost-effective technical stack to build your own reverse geocoding server using public open geospatial data.

## Introduction
Determining whether a coordinate (a point) is inside an administrative boundary (a polygon) relies on the **Ray Casting Algorithm** (Point-in-Polygon test).

One might assume that storing these boundary polygons in a Relational Database (RDB) like MySQL or PostgreSQL and querying them using spatial/geometry extensions would be a quick and easy solution. While this is simple to implement, it suffers from poor response times—especially when the active dataset exceeds memory caches, causing disk I/O bottlenecks that can crash the server during traffic spikes. Conversely, provisioning extremely high-RAM instances to fit the entire spatial database into memory defeats the purpose of using a heavy RDB in the first place.

Our approach loads the raw Shapefiles, constructs highly optimized spatial indices directly in system memory, and serves requests with sub-millisecond latency. Although building such a system from scratch might sound complex, the modern open-source ecosystem provides powerful tools that make this highly achievable.

This project implements a reverse geocoding server that searches down to the land lot (premise/parcel) polygon level using public Shapefile data. It introduces:
- A high-performance server architecture using the right languages and geometry libraries.
- An automated data converter and preprocessor.
- An interactive, cross-platform polygon editing tool to fix broken source data.
- Key considerations and best practices for governments when releasing geospatial public datasets.

## Implementation
Developing a system like this was made significantly more approachable thanks to the assistance of Large Language Models (LLMs).

### Tech Stack

#### 1. Rust (Server)
While modern servers are often written in Java/Kotlin, Python, or Node.js, handling portal-scale high-concurrency spatial traffic with these runtimes requires substantial hardware resources. To handle massive traffic, industries historically resorted to C/C++ modules (e.g., Apache/Nginx modules) or Erlang. Go (Golang) is also a popular choice today due to its low learning curve and superior performance compared to Java.

However, for a system requiring absolute predictability, maximum throughput, and zero-cost abstractions, **Rust** stands out as the ultimate choice. Once compiled, Rust guarantees compile-time memory safety, thread safety, and eliminates unpredictable garbage collection (GC) pauses. While Rust has a steep learning curve, using LLMs makes writing idiomatic, high-performance concurrent Rust code much more accessible.

Here is an example of parallel loading of hierarchical administrative boundary polygons using rayon's `par_iter()`:

```rust
config.districts
    .par_iter()
    .try_for_each(|district| {
        config.district_par
            .par_iter()
            .try_for_each(|name|
                self.load_district_par_polygons(
                    config,
                    district, 
                    name,
                    config.debug, 
                    &config.debug_name,
                )
            )?;
        config.district_par_any
            .par_iter()
            .try_for_each(|name|
                self.load_district_par_any_polygons(
                    config,
                    district, 
                    name,
                    config.debug, 
                    &config.debug_name,
                )
            )
    })?;
```

#### 2. S2Geometry (Spatial Index & Spherical Geometry)
Created by Eric Veach at Google, **S2** is the core geometry library powering Google Maps.

> *“S2 is a library for spherical geometry that aims to have the same robustness, flexibility, and performance as the very best planar geometry libraries.”*

By projecting the Earth onto a 3D sphere rather than a flat 2D plane, S2 provides incredible precision, robust indexing, and blazing-fast spatial queries.

#### 3. Autocxx (Rust-to-C++ Binding)
Since the S2Geometry library is not fully ported to native Rust, we use `autocxx` to safely and seamlessly call the original, battle-tested C++ S2Geometry library from Rust.

#### 4. Configuration (TOML)
Flexible configurations are essential for a portable application. Compared to XML (which can be verbose and hard to read), JSON, or YAML, **TOML** offers a clean, developer-friendly syntax. 

The `config.toml` file defines how geospatial datasets are parsed, structured into hierarchical levels, and queried:

```toml
[search.shapefile]
path = "./data/converted"

[search.shapefile.attributes]
"TL_SCCO_CTPRVN" = { level = 1, names = ["CTPRVN_CD", "CTP_KOR_NM", "CTP_ENG_NM"]}
"TL_SCCO_SIG" = { level = 2, names = ["SIG_CD", "SIG_KOR_NM", "SIG_ENG_NM"]}
"TL_SCCO_EMD" = { level = 3, names = ["EMD_CD", "EMD_KOR_NM", "EMD_ENG_NM"]}
"TL_SCCO_LI" = { level = 4, names = ["LI_CD", "LI_KOR_NM", "LI_ENG_NM"]}
"AL_D002_" = { level = 5, names = ["A3", "A4", "A5"]}

[search]
districts = [
  #"11000",    
  #"41000",    
  #"46000", 
  "36000",   
]

hierarchies = [
  "TL_SCCO_CTPRVN",
]

district_par = [
  "TL_SCCO_SIG",
  "TL_SCCO_EMD",
  "TL_SCCO_LI",
]

district_par_any = [
  "AL_D002_"
]

[rest]
port = 8080
host = "localhost"

[grpc]
port = 8090
host = "localhost"
```

#### 5. Electron (Polygon Geometry Editor)
When dealing with real-world public GIS data, you will inevitably run into geometry errors (e.g., self-intersections, degenerate loops). To fix these, we need an editing tool.
Rather than building a complex native UI, leveraging web technologies (HTML/CSS/JS) is the most efficient route.

**Electron** is an open-source framework that lets developers build cross-platform desktop applications (Windows, macOS, Linux) using web standards. We built a visual editor in Electron using LLM-assisted "Vibe Coding" to load Shapefiles, inspect broken geometry, make visual corrections, and save them back.

#### 6. Postman (API Testing)
We use Postman to quickly test and validate both our REST and gRPC API endpoints.

---

### Data
This project is configured for **South Korea** as the reference region.
Administrative divisions in Korea follow a strict hierarchy:
`Level 0 (Country: South Korea) > Level 1 (Province/City: Gyeonggi-do) > Level 2 (District/County: Suji-gu, Yongin-si) > Level 3 (Town/Neighborhood: Shinbong-dong) > Level 4 (Sub-district/Village: Li) > Level 5 (Land Lot/Premise Polygon)`.

- **Administrative Boundary Polygons (Level 1–4):** Can be downloaded by province from the [Address-based Industry Support Service](https://business.juso.go.kr/addrlink/adresInfoProvd/guidance/provdAdresInfo.do) (under Electronic Map Download > Road Name Address Electronic Map).
- **Land Lot (Premise/Cadastral) Polygons (Level 5):** Can be downloaded from [V-World (Digital Twin Country > Continuous Cadastral Map)](https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?svcCde=NA&dsId=23).

#### Administrative Boundary Shapefiles
Coordinate System: **ITRF2000**, Reference Ellipsoid: **GRS80**, Projection: **UTM**, Semi-major axis: **6,378,137m**.
Example for Sejong City (`36000.zip`):
```
Province (Level 1):       TL_SCCO_CTPRVN.shp 
City/County/Dist (L2):    TL_SCCO_SIG.shp 
Town/Neighborhood (L3):   TL_SCCO_EMD.shp 
Village/Li (Level 4):     TL_SCCO_LI.shp 
```

#### Land Lot (Premise) Shapefiles
Coordinate System: **EPSG:5186 (GRS80)**.
Example for Sejong City:
```
AL_D002_36_20250504.zip
```

#### Issues with Public GIS Data
Raw government geospatial data comes with several practical issues that impede out-of-the-box usage:
1. **Legacy Encoding:** Often encoded in `EUC-KR` instead of `UTF-8`.
2. **Inconsistent Map Projections:** Different agencies publish files in different spatial coordinate systems (e.g., EPSG:5179 vs. EPSG:5186).
3. **Massive File Sizes:** Huge land-lot files can exceed shapefile vertex limits or cause massive load times if not split.

##### Raw Data Conversion
We provide a Go-based preprocessor (`convert.go`) that converts text encoding from `EUC-KR` to `UTF-8`, reprojects coordinates (e.g., EPSG:5179/5186) to WGS84 (`EPSG:4326`), and splits files that exceed 12MB (or 600,000 vertices) into smaller parts to optimize memory mapping:
```bash
go run convert.go -h
../data/source/46000/AL_D002_46_20250204.shp -> ../data/all_converted/46000/AL_D002_46_20250204_part1.shp
../data/all_converted/46000/AL_D002_46_20250204_part2.shp
../data/all_converted/46000/AL_D002_46_20250204_part3.shp
...
../data/all_converted/46000/AL_D002_46_20250204_part25.shp
../data/source/46000/TL_SCCO_CTPRVN.shp -> ../data/all_converted/46000/TL_SCCO_CTPRVN.shp
```

##### S2Geometry Polygon Validation
S2 is extremely strict about geometry validity. Invalid polygons will fail to build or crash during load, so they must be filtered out or repaired. Common topological errors in raw datasets include:
- **Degenerate Edges (Duplicate Vertices):** `ERROR Edge 1990 is degenerate (duplicate vertex)`
- **Self-intersections (Crossing Edges):** `ERROR Edge 455 crosses edge 457`
- **Inverted Curvatures (Winding Order Issues)**
- **Empty Polygons**

Run validation tests:
```bash
cargo test -- --nocapture test_search_polygon   
cargo test -- --nocapture test_load_polygon   
```

##### Visual Geometry Repair (Editor)
To fix these topology issues, we built an Electron-based editor. You can load a specific faulty shapefile and patch coordinates interactively:
```bash
npm run start -- --shapefile-path ../data/converted/46000/TL_SCCO_EMD.shp     
npm run start -- --shapefile-path ../data/all_converted/46000/AL_D002_46_20250204_6_part22.shp 
# Keys: 'u' to update selected point, 'x' to deselect, 's' to save changes
```

Here are some examples of geometry errors found in the raw public source files:
<div align="center">
  <img width="180" alt="Self-intersection error" src="https://github.com/user-attachments/assets/4f15da3f-418b-4a1a-b336-f4d81a1da3ca" />
  <img width="180" alt="Duplicate vertex error" src="https://github.com/user-attachments/assets/453193a6-2774-431c-8066-fde5e755e8f9" />
  <img width="180" alt="Overlapping edge error" src="https://github.com/user-attachments/assets/b0a97660-e15a-4de2-97ce-762eff70be32" />
  <img width="180" alt="Spike/Degenerate loop error" src="https://github.com/user-attachments/assets/e69b592d-acb3-4777-8eac-4ab608588038" />
</div>

---

### Running the Server
```bash
cargo run
    Running `target/debug/tarantula`
load_district_polygons 36000 TL_SCCO_SIG elapsed: 129.599833ms
load_district_polygons 36000 TL_SCCO_EMD elapsed: 297.3095ms
load_district_polygons 36000 TL_SCCO_LI elapsed: 311.834583ms
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 0 crosses edge 2
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part4.shp load_polygon 5-2 error_code: 1
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 18 crosses edge 21
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part4.shp load_polygon 38-4 error_code: 1
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 5 crosses edge 21
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part4.shp load_polygon 81-1 error_code: 1
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part4.shp load_polygon 138-2 error_code: 3
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 0 crosses edge 3
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part2.shp load_polygon 524 error_code: 1
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 767 crosses edge 770
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part3.shp load_polygon 24-1 error_code: 1
/github/s2geometry/src/s2/s2loop.cc:131 ERROR Edge 4 crosses edge 8
/github/tarantula/data/all_converted/36000/AL_D002_36_20250504_part3.shp load_polygon 134-1 error_code: 1
load_district_par_any_polygons 36000 AL_D002_ elapsed: 9.017407292s
Search load elapsed: 9.467750875s
grpc server listening on [::1]:8090
rest api server listening on [::1]:8080
```

#### REST API Response Sample
<img width="820" alt="REST API Screenshot" src="https://github.com/user-attachments/assets/323f435f-dd47-4d9d-add7-0b53ec4b7a4f" />  

#### gRPC API Response Sample
<img width="820" alt="gRPC API Screenshot" src="https://github.com/user-attachments/assets/2f268f2d-e2c6-4d6b-9c75-72e1b78df5f6" />

---

### Performance Benchmark
Tested on an **Apple M1 Pro (32GB RAM)** using the Apache Benchmark tool (`ab`):

#### REST API Benchmark
```bash
ab -n 1000000 -c 60 "http://localhost:8080/tarantula?lon=126.973826366&lat=37.532190912"
```
```text
Server Software:
Server Hostname:        localhost
Server Port:            8080

Document Path:          /tarantula?lon=126.973826366&lat=37.532190912
Document Length:        208 bytes

Concurrency Level:      60
Time taken for tests:   59.478 seconds
Complete requests:      1000000
Failed requests:        0
Total transferred:      317000000 bytes
HTML transferred:       208000000 bytes
Requests per second:    16812.83 [#/sec] (mean)
Time per request:       3.569 [ms] (mean)
Time per request:       0.059 [ms] (mean, across all concurrent requests)
Transfer rate:          5204.75 [Kbytes/sec] received

Connection Times (ms)
              min  mean[+/-sd] median   max
Connect:        0    0   0.8      0     116
Processing:     0    3   3.6      3     125
Waiting:        0    3   3.5      3     125
Total:          0    4   3.6      3     125
```
*Successfully handling over **16,800 requests per second** with a mean response latency of just **0.059 ms** per concurrent request under high load.*

#### gRPC API Benchmark
```bash
bash pref_grpc.sh pref_grpc_req.json
```

---

## Conclusion

### 1. Recommendations for Governments Releasing Open Spatial Data
To foster innovation and maximize the economic and social utility of public data, public organizations should consider the following:
- **Data Format & Standardization:** Standardize text encoding to `UTF-8` and coordinates to `EPSG:4326 (WGS84)`. Providing unified, modern projections minimizes redundant preprocessing.
- **Ready-to-Use Usability:** Establish validation pipelines prior to publishing datasets. Delivering topology-cleansed, production-ready files prevents thousands of hours of duplicate debugging efforts by developers.
- **Technical Stability of APIs:** If offering official public APIs, guarantee technical uptime and failovers. If hosting public endpoints is too costly or unsustainable, open-sourcing clean raw datasets is a highly effective alternative.
- **Active Feedback Loops:** Setup public issue trackers (e.g., GitHub Repos) to gather community feedback on geometry errors, update intervals, and data completeness.

### 2. Summary of the Recommended Tech Stack
- **Server:** Rust (provides unmatched throughput, memory safety, and deterministic low latency).
- **Spatial Index Engine:** C++ S2Geometry via Rust `autocxx` bindings.
- **Visual Editing & Debugging Tool:** Electron (cross-platform desktop shell for visual web interfaces).

---

## Quick Start
Watch the quick start demo on YouTube: [Tarantula Demo](https://www.youtube.com/watch?v=zJDaM0ofj6Y)

### Prerequisites
- Install **Rust/Cargo**, **Go**, and **Homebrew**.
- Ensure `protoc` is installed (required by the macOS build scripts):
```bash
brew install protobuf pkg-config proj
```
*Note: Make sure to download and place the raw shapefiles before launching the server, otherwise the startup data loading phase will fail.*

### 1) Extract Raw Data
Prepare Sejong City data inside `data/source/36000.zip` (administrative boundary map) and `AL_D002_36_20250504.zip` (cadastral land lot map), then extract them:
```bash
bash data.sh
```

### 2) Preprocessing: Encodings, Projections, and Splitting
Run the Go script to convert Shapefiles into modern formats under the `data/converted` directory:
```bash
cd shape_convert
go run convert.go
```

### 3) Launch the Server
```bash
cargo run
```
> 💡 *Troubleshooting:*
> - If compilation fails with `Could not find protoc`, ensure `brew install protobuf` has run successfully.
> - If you see `failed to load search` or `MissingDbf` runtime errors, double-check that your data extraction and conversion steps were completed successfully.

### 4) Run the Desktop Debugging Editor
```bash
cd shape_edit
npm install
npm run start
```
