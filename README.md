# ALICE-Camera-SaaS

Image signal processing SaaS built on the ALICE-Camera engine. Provides ISP pipeline processing, camera calibration, demosaicing, and preset management via REST API.

## Architecture

```
Client --> API Gateway (8110) --> Core Engine (8116)
```

- **API Gateway**: Authentication, rate limiting, request proxying
- **Core Engine**: ISP pipeline, calibration engine, preset manager

## Features

- Full ISP pipeline (demosaic, denoise, AWB, AE, sharpening, tone mapping)
- RAW (DNG, RGGB, GBRG) and processed image input
- Lens distortion correction and geometric calibration
- Color science: ICC profiles, gamut mapping, LUT application
- Noise reduction (BM3D, NLM, wavelet)
- HDR merge from bracketed exposures
- Preset library for common camera profiles

## API Endpoints

### Core Engine (port 8116)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check with uptime and stats |
| POST | `/api/v1/camera/process` | Run ISP pipeline on raw/image data |
| POST | `/api/v1/camera/pipeline` | Run custom pipeline with stage config |
| POST | `/api/v1/camera/calibrate` | Calibrate camera from checkerboard images |
| GET | `/api/v1/camera/presets` | List available ISP presets |
| GET | `/api/v1/camera/stats` | Operational statistics |

### API Gateway (port 8110)

Proxies all `/api/v1/*` routes to the Core Engine with JWT/API-Key auth and token-bucket rate limiting.

## Quick Start

```bash
# Core Engine
cd services/core-engine
CAMERA_ADDR=0.0.0.0:8116 cargo run --release

# API Gateway
cd services/api-gateway
GATEWAY_ADDR=0.0.0.0:8110 CORE_ENGINE_URL=http://localhost:8116 cargo run --release
```

## Example Request

```bash
curl -X POST http://localhost:8116/api/v1/camera/process \
  -H "Content-Type: application/json" \
  -d '{"raw_b64":"...","bayer_pattern":"RGGB","preset":"cinema-d65","output_format":"jpeg"}'
```

## License

AGPL-3.0-or-later. SaaS operators must publish complete service source code under AGPL-3.0.
