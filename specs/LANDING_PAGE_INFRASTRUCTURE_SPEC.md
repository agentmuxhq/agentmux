# AgentMux Landing Page Infrastructure Spec

## Overview

Create a **SolidJS landing page** for `agentmux.ai` with three environments (dev, qa, prod), following the same infrastructure pattern used by **asafebgi** and **stratum** -- S3 + CloudFront + Route53 + ACM via AWS CDK.

Currently `agentmux.ai` is a **301 redirect** to `asafebgi.com` via the shared `asafebgi-redirects-prod` CloudFront distribution (`E1PDHJVBNA6L03`). This spec replaces that redirect with a full landing page stack.

---

## Current State

| Resource | Current Value |
|----------|--------------|
| Domain | `agentmux.ai` |
| Hosted Zone | `Z05724973E1L5SZ1VHC5Q` |
| DNS A Record | Points to `dkk6bvhqhmtib.cloudfront.net` (redirect distro) |
| ACM Certificate | `agentmux.asaf.cc` only -- **no cert for `agentmux.ai`** |
| CloudFront | Shared redirect distro (aliases: `asaf.cc`, `agentmux.ai`, `a5af.com`) |
| Landing Page | None -- currently redirects to `asafebgi.com` |

---

## Target Architecture

```
  DEV (local)                     QA / PROD (cloud)
  ────────────                    ─────────────────
  localhost:4600                  qa.agentmux.ai / agentmux.ai
       │                                    │
  Vite dev server                    Route53 A Record
       │                                    │
  Hot reload                     ┌──────────▼──────────┐
       │                         │  CloudFront          │
  Points to cloud                │  - HTTPS (ACM cert)  │
  backend APIs ──────────────►   │  - SPA routing       │
  (dev Lambda/API GW)            │  - HTTP Auth (QA)    │
                                 └──────────┬──────────┘
                                            │
                                  Origin Access Control
                                            │
                                 ┌──────────▼──────────┐
                                 │  S3 Bucket           │
                                 │  agentmux-landing-*  │
                                 │  Static SolidJS build│
                                 └─────────────────────┘
```

---

## Environment Layout

| Environment | Frontend | Backend | Domain | CloudFront | HTTP Auth | Purpose |
|-------------|----------|---------|--------|-----------|-----------|---------|
| **dev** | Local Vite (`localhost:4600`) | Cloud (dev Lambda/API GW) | None | No | No | Active development, hot reload |
| **qa** | S3 static | Cloud (qa Lambda/API GW) | `qa.agentmux.ai` | Yes | Yes (unified-auth) | Pre-release review |
| **prod** | S3 static | Cloud (prod Lambda/API GW) | `agentmux.ai` + `www.agentmux.ai` | Yes | No | Public-facing |

> **Dev note:** Dev frontend runs locally via `vite --host 0.0.0.0 --port 4600`. No S3 bucket or CloudFront for dev. Backend APIs are the existing cloud dev endpoints.

---

## Pre-Requisites (Before CDK Deploy)

### 1. ACM Certificate for `agentmux.ai`

**Needed:** A wildcard certificate covering all environments.

```
Request Certificate:
  Domain: agentmux.ai
  SANs:   *.agentmux.ai, www.agentmux.ai
  Region: us-east-1 (required for CloudFront)
  Validation: DNS (auto-validate via Route53 hosted zone Z05724973E1L5SZ1VHC5Q)
```

**Steps:**
```bash
aws acm request-certificate \
  --domain-name "agentmux.ai" \
  --subject-alternative-names "*.agentmux.ai" "www.agentmux.ai" \
  --validation-method DNS \
  --region us-east-1

# Then add CNAME validation records to Route53
# CDK can also handle this via DnsValidatedCertificate
```

### 2. Remove `agentmux.ai` from Redirect Distribution

Before the new CloudFront distribution can use `agentmux.ai` as an alias, remove it from the shared redirect distro (`E1PDHJVBNA6L03`).

**Steps:**
1. Update redirect distribution `E1PDHJVBNA6L03` to remove `agentmux.ai` from aliases (keep `asaf.cc` and `a5af.com`)
2. Delete the current Route53 A record for `agentmux.ai` pointing to the redirect distro
3. CDK will create new A records pointing to the new per-environment CloudFront distributions

### 3. Remove `agentmux.ai` from asafebgi Redirect Construct

The asafebgi CDK stack's `RedirectConstruct` lists `agentmux.ai` in its `redirectDomains`. Remove it from the asafebgi prod config:

```typescript
// asafebgi/lib/config/environments.ts - prod config
// REMOVE: redirectDomains: ['asaf.cc', 'agentmux.ai'],
// CHANGE TO: redirectDomains: ['asaf.cc'],
```

Deploy the asafebgi stack change first to release the domain from the redirect distro.

---

## Project Structure

Create a `landing/` directory inside the agentmux repo:

```
agentmux/
├── landing/                          # NEW - SolidJS landing page
│   ├── src/
│   │   ├── index.tsx                # App entry point
│   │   ├── App.tsx                  # Root component
│   │   ├── components/
│   │   │   ├── Hero.tsx             # Hero section
│   │   │   ├── Features.tsx         # Feature highlights
│   │   │   ├── Download.tsx         # Download / CTA section
│   │   │   ├── Footer.tsx           # Footer
│   │   │   └── Nav.tsx              # Navigation bar
│   │   ├── styles/
│   │   │   └── global.css           # Global styles (Tailwind)
│   │   └── assets/
│   │       ├── logo.svg             # AgentMux logo
│   │       └── screenshots/         # Product screenshots
│   ├── public/
│   │   ├── favicon.ico
│   │   ├── robots.txt
│   │   └── site.webmanifest
│   ├── index.html                   # HTML entry
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts               # Vite + SolidJS
│   ├── tailwind.config.ts           # Tailwind CSS
│   └── postcss.config.js
│
├── landing-cdk/                      # NEW - CDK infrastructure
│   ├── bin/
│   │   └── landing.ts               # CDK app entry point
│   ├── lib/
│   │   ├── landing-stack.ts         # Main stack
│   │   ├── config/
│   │   │   └── environments.ts      # dev/qa/prod configs
│   │   └── constructs/
│   │       ├── frontend.ts          # S3 + CloudFront + Route53
│   │       └── outputs.ts           # Stack outputs
│   ├── package.json
│   ├── tsconfig.json
│   └── cdk.json
│
├── infra/                            # EXISTING - webhook infra (unchanged)
│   └── cdk/
│       └── lib/
│           └── agentmux-webhook-stack.ts
├── frontend/                         # EXISTING - Tauri app frontend (unchanged)
└── src-tauri/                        # EXISTING - Tauri Rust backend (unchanged)
```

---

## SolidJS Landing Page

### Tech Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| SolidJS | 1.9+ | UI framework |
| Vite | 6.x | Build tool |
| TypeScript | 5.x | Type safety |
| Tailwind CSS | 4.x | Styling |
| @solidjs/router | 0.15+ | Client-side routing (if needed) |

### Landing Page Sections

1. **Nav** - Logo, links (Features, Download, GitHub), dark theme
2. **Hero** - Tagline ("AI-Native Terminal Multiplexer"), product screenshot, CTA button
3. **Features** - Grid of key features:
   - Multi-pane terminal with AI agent integration
   - 100% Rust backend for performance
   - Built-in Claude AI integration
   - Monaco code editor, system metrics, web views
   - Cross-platform (Windows, macOS, Linux)
4. **Download** - Platform-specific download links (detect OS), link to GitHub Releases
5. **Footer** - GitHub link, license (Apache-2.0), version

### Build Configuration

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import solidPlugin from 'vite-plugin-solid';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [solidPlugin(), tailwindcss()],
  build: {
    target: 'esnext',
    outDir: 'dist',
  },
});
```

### package.json

```json
{
  "name": "@a5af/agentmux-landing",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "vite --host 0.0.0.0 --port 4600",
    "dev:local": "VITE_API_URL=http://localhost:3000 vite --host 0.0.0.0 --port 4600",
    "build": "vite build",
    "build:qa": "VITE_API_URL=https://api-qa.agentmux.ai vite build",
    "build:prod": "VITE_API_URL=https://api.agentmux.ai vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "solid-js": "^1.9.0"
  },
  "devDependencies": {
    "vite": "^6.0.0",
    "vite-plugin-solid": "^2.11.0",
    "@tailwindcss/vite": "^4.0.0",
    "tailwindcss": "^4.0.0",
    "typescript": "^5.7.0"
  }
}
```

---

## CDK Infrastructure

### Environment Configuration

```typescript
// landing-cdk/lib/config/environments.ts

export interface LandingEnvironmentConfig {
  stage: 'dev' | 'qa' | 'prod';
  stackName: string;
  domainNames: string[];
  certificateArn: string;
  hostedZoneId: string;
  hostedZoneName: string;
  bucketName: string;
  useHttpAuth: boolean;     // QA only
}

// Dev has NO cloud infrastructure -- local Vite dev server only.
// Backend APIs point to cloud dev endpoints via VITE_API_URL env var.

export const environments: Record<string, LandingEnvironmentConfig> = {
  qa: {
    stage: 'qa',
    stackName: 'agentmux-landing-qa',
    domainNames: ['qa.agentmux.ai'],
    certificateArn: '<NEW_WILDCARD_CERT_ARN>',
    hostedZoneId: 'Z05724973E1L5SZ1VHC5Q',
    hostedZoneName: 'agentmux.ai',
    bucketName: 'agentmux-landing-qa',
    useHttpAuth: true,      // HTTP Basic Auth via unified-auth Lambda@Edge
  },
  prod: {
    stage: 'prod',
    stackName: 'agentmux-landing-prod',
    domainNames: ['agentmux.ai', 'www.agentmux.ai'],
    certificateArn: '<NEW_WILDCARD_CERT_ARN>',
    hostedZoneId: 'Z05724973E1L5SZ1VHC5Q',
    hostedZoneName: 'agentmux.ai',
    bucketName: 'agentmux-landing-prod',
    useHttpAuth: false,
  },
};
```

### Frontend Construct (S3 + CloudFront + Route53)

```typescript
// landing-cdk/lib/constructs/frontend.ts

export class LandingFrontend extends Construct {
  public readonly distribution: cloudfront.Distribution;
  public readonly bucket: s3.Bucket;

  constructor(scope: Construct, id: string, props: LandingFrontendProps) {
    super(scope, id);

    const { config } = props;

    // S3 bucket for static files
    this.bucket = new s3.Bucket(this, 'Bucket', {
      bucketName: config.bucketName,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      removalPolicy: config.stage === 'prod'
        ? cdk.RemovalPolicy.RETAIN
        : cdk.RemovalPolicy.DESTROY,
      autoDeleteObjects: config.stage !== 'prod',
    });

    // ACM certificate
    const certificate = acm.Certificate.fromCertificateArn(
      this, 'Certificate', config.certificateArn
    );

    // Import shared CORS + Security response headers policy
    const responseHeadersPolicy = cloudfront.ResponseHeadersPolicy
      .fromResponseHeadersPolicyId(
        this, 'HeadersPolicy',
        cdk.Fn.importValue('InfrastructureSecurityHeadersPolicyId')
      );

    // HTTP Auth Lambda@Edge for QA (unified-auth v30)
    const edgeLambdas = config.useHttpAuth ? [{
      functionVersion: lambda.Version.fromVersionArn(
        this, 'UnifiedAuth',
        'arn:aws:lambda:us-east-1:050544946291:function:infrastructure-unified-auth:30'
      ),
      eventType: cloudfront.LambdaEdgeEventType.VIEWER_REQUEST,
    }] : undefined;

    // CloudFront distribution
    this.distribution = new cloudfront.Distribution(this, 'Distribution', {
      comment: `AgentMux Landing ${config.stage}`,
      defaultRootObject: 'index.html',
      domainNames: config.domainNames,
      certificate,
      priceClass: cloudfront.PriceClass.PRICE_CLASS_100,
      httpVersion: cloudfront.HttpVersion.HTTP2_AND_3,
      defaultBehavior: {
        origin: origins.S3BucketOrigin.withOriginAccessControl(this.bucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
        responseHeadersPolicy,
        compress: true,
        ...(edgeLambdas && { edgeLambdas }),
      },
      // SPA routing - rewrite 403/404 to index.html
      errorResponses: [
        {
          httpStatus: 403,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
          ttl: cdk.Duration.minutes(5),
        },
        {
          httpStatus: 404,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
          ttl: cdk.Duration.minutes(5),
        },
      ],
    });

    // Route53 DNS records
    const hostedZone = route53.HostedZone.fromHostedZoneAttributes(
      this, 'HostedZone', {
        hostedZoneId: config.hostedZoneId,
        zoneName: config.hostedZoneName,
      }
    );

    for (const domainName of config.domainNames) {
      const recordName = domainName === config.hostedZoneName
        ? undefined  // apex domain
        : domainName.replace(`.${config.hostedZoneName}`, '');

      new route53.ARecord(this, `DnsRecord-${domainName}`, {
        zone: hostedZone,
        recordName,
        target: route53.RecordTarget.fromAlias(
          new targets.CloudFrontTarget(this.distribution)
        ),
      });
    }
  }
}
```

### CDK Entry Point

```typescript
// landing-cdk/bin/landing.ts
const app = new cdk.App();
const envName = app.node.tryGetContext('env') || 'dev';
const config = environments[envName];

new AgentMuxLandingStack(app, config.stackName, {
  config,
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: 'us-east-1',
  },
  tags: {
    Project: 'agentmux-landing',
    Environment: config.stage,
    ManagedBy: 'cdk',
  },
});
```

---

## Deployment

### Local Dev

```bash
cd /workspace/agentmux/landing

# Start Vite dev server (hot reload)
npm run dev
# → http://localhost:4600

# Points to cloud dev backend via env var:
# VITE_API_URL=https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com
```

Accessible via Traefik proxy at `http://agentmux-landing-agent1.test` (port 4600).

### Using `deploy` CLI (QA/Prod)

```bash
# QA
deploy run --env qa --component landing

# Prod
deploy run --env prod --component landing
```

### Manual CDK (Infrastructure Changes)

```bash
cd /workspace/agentmux/landing-cdk

# Synth & diff
cdk synth --context env=qa
cdk diff --context env=qa

# Deploy
cdk deploy --context env=qa                              # qa
cdk deploy --context env=prod                            # prod (requires approval)
```

### Frontend Deploy (S3 Sync + CDN Invalidation)

```bash
cd /workspace/agentmux/landing

# Build
npm run build

# Sync to S3 (qa example)
aws s3 sync dist/ s3://agentmux-landing-qa/ --delete

# Invalidate CloudFront cache
aws cloudfront create-invalidation \
  --distribution-id <DIST_ID> \
  --paths "/*"
```

---

## Implementation Order

### Phase 1: Pre-Requisites
1. Request ACM wildcard certificate for `agentmux.ai` + `*.agentmux.ai`
2. Wait for DNS validation (auto via Route53)
3. Remove `agentmux.ai` from asafebgi redirect construct
4. Deploy updated asafebgi stack to release the domain
5. Remove `agentmux.ai` alias from shared redirect CloudFront distro

### Phase 2: CDK Infrastructure
1. Create `landing-cdk/` directory with CDK project
2. Implement `LandingFrontend` construct
3. Deploy qa stack first (`agentmux-landing-qa`)
4. Verify `qa.agentmux.ai` resolves correctly
5. Deploy prod stack (`agentmux-landing-prod`)
6. Verify `agentmux.ai` serves the landing page

### Phase 3: SolidJS Landing Page
1. Scaffold SolidJS project in `landing/`
2. Build initial landing page with hero, features, download sections
3. Deploy to dev, iterate on design
4. Promote to qa for review
5. Promote to prod

### Phase 4: Deploy CLI Integration
1. Add `agentmux-landing` to deploy CLI project registry
2. Configure S3 sync + CloudFront invalidation per environment
3. Test full deploy pipeline: `deploy run --env dev --component landing`

---

## Cost Estimate (Monthly)

| Resource | QA | Prod | Total |
|----------|-----|------|-------|
| S3 (static files, <1MB) | $0.01 | $0.01 | $0.02 |
| CloudFront (light traffic) | $0.10 | $2.00 | $2.10 |
| Route53 (hosted zone, already exists) | -- | -- | $0.50 |
| ACM Certificate | Free | Free | Free |
| Dev (local only) | -- | -- | $0.00 |
| **Total** | | | **~$2.60/month** |

---

## Notes

- **No existing landing page** -- the repo's `docs/` is a Docusaurus docs site (still Wave Terminal branded), and `index.html` is the Tauri app shell. Landing page is built from scratch.
- The existing `infra/cdk/` (webhook stack) is **not touched** -- it remains separate infrastructure for the desktop app's agent communication system
- The landing page is purely static (no backend Lambda needed initially -- download links point to GitHub Releases)
- The Tauri desktop app (`frontend/` + `src-tauri/`) is completely separate from the landing page
- **Dev = local only** -- `vite --port 4600` with hot reload. Backend APIs use cloud dev endpoints via `VITE_API_URL` env var. No S3/CloudFront for dev.
- Dev accessible via Traefik at `http://agentmux-landing-agent1.test` (port 4600)
- Follow AWS naming conventions: human-readable bucket/distribution names
