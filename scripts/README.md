# OpenID4VC Backend - End-to-End Testing Scripts

This directory contains comprehensive testing resources for the OpenID4VC Backend API.

## Files

- **`e2e-flow.sh`** - Interactive bash script for complete end-to-end flow
- **`curl-commands.sh`** - Reference documentation with all curl commands
- **`OpenID4VC-Backend.postman_collection.json`** - Postman collection for API testing

## Quick Start

### 1. Using the Bash Script

```bash
# Make executable
chmod +x scripts/e2e-flow.sh

# Run complete flow
cd scripts && ./e2e-flow.sh all

# Or run individual steps
./e2e-flow.sh health
./e2e-flow.sh tenant "My Org" "my-org"
./e2e-flow.sh apikey <tenant_id> "My Key" "issuer_manager"
./e2e-flow.sh creddef <api_key> "MyCredential" "jwt_vc_json"
./e2e-flow.sh offer <api_key> <cred_def_id>
./e2e-flow.sh revoke <admin_api_key> <credential_id> "Key compromise"
```

### 2. Using Postman

1. Import `OpenID4VC-Backend.postman_collection.json`
2. Create an environment with these variables:
   - `base_url`: `http://localhost:3000`
   - `master_api_key`: (optional, for master key auth)
   - `dev_tenant_id`: `00000000-0000-0000-0000-000000000000`
   - `dev_role`: `super_admin`
   - `dev_principal`: `admin`
3. Run requests in order - IDs are automatically captured

### 3. Using Curl Commands

See `curl-commands.sh` for copy-paste ready commands.

## Flow Overview

The complete tenant→API key→issuance→revoke flow:

```
┌─────────────────┐
│ 1. Health Check │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 2. Create Tenant│ (SuperAdmin)
└────────┬────────┘
         │ tenant_id
         ▼
┌──────────────────────────┐
│ 3. Create Issuer API Key │ (SuperAdmin)
└────────┬─────────────────┘
         │ api_key (issuer_manager)
         ▼
┌─────────────────────────────┐
│ 4. Create Credential Def    │ (IssuerManager)
└────────┬────────────────────┘
         │ credential_def_id
         ▼
┌──────────────────────────┐
│ 5. Create Credential Offer│ (IssuerManager)
└────────┬─────────────────┘
         │ offer_url, session_id
         ▼
┌─────────────────────────┐
│ 6. Wallet OID4VCI Flow   │ (Public endpoints)
└────────┬────────────────┘
         │ credential_id
         ▼
┌──────────────────────────────┐
│ 7. Create TenantAdmin API Key│ (SuperAdmin)
└────────┬─────────────────────┘
         │ api_key (tenant_admin)
         ▼
┌──────────────────────────┐
│ 8. Revoke Credential     │ (TenantAdmin)
└──────────────────────────┘
```

## Authentication Methods

The API supports three authentication methods:

### 1. Master API Key (Production)
```bash
curl -H "X-Api-Key: your-master-key" ...
```

### 2. Generated API Keys (Per-tenant)
```bash
# Keys returned when creating API keys (format: ok_<uuid>)
curl -H "X-Api-Key: ok_550e8400-e29b-41d4-a716-446655440000" ...
```

### 3. Development Headers (Local Testing)
```bash
curl -H "X-Tenant-Id: <uuid>" \
     -H "X-Role: super_admin" \
     -H "X-Principal: admin" ...
```

## Role Permissions

| Role | Permissions |
|------|-------------|
| `super_admin` | Full access |
| `tenant_admin` | Credential definitions, revocation |
| `issuer_manager` | Create offers, issue credentials |
| `verifier` | Create verification requests |
| `readonly` | Read-only access |
| `api_client` | Webhook access only |

## Environment Variables

For the bash script, you can set these variables:

```bash
export OID4VC_BACKEND__SERVER__BASE_URL="http://localhost:3000"
export OID4VC_BACKEND__AUTH__MASTER_API_KEY="your-master-key"
export DEV_TENANT_ID="00000000-0000-0000-0000-000000000000"
export DEV_ROLE="super_admin"
export DEV_PRINCIPAL="admin"
```

## Troubleshooting

### API Not Responding
```bash
# Check if server is running
curl http://localhost:3000/health
# Expected: {"status":"ok"}
```

### Authentication Errors
- Verify your API key is correct and active
- Check that the key has the required role
- For dev mode, ensure `X-Tenant-Id`, `X-Role`, and `X-Principal` headers are set

### Database Errors
- Verify PostgreSQL is running: `docker compose up -d postgres`
- Check migrations are applied: see `/migrations/` directory

### Missing Variables
When using the bash script, run steps in order so variables are captured:
```bash
./e2e-flow.sh all
```

## Testing the Wallet Flow

After creating a credential offer, you can test the OID4VCI flow:

1. Get the `offer_url` from the Create Offer response
2. Use a wallet app that supports OID4VCI
3. Scan or input the offer URL
4. Complete the issuance flow

For manual testing, use the public endpoints in the Postman collection under "05 - OID4VCI Protocol".

## Complete Example

```bash
# 1. Set base URL
export BASE_URL="http://localhost:3000"

# 2. Create tenant (capture output)
TENANT=$(curl -s -X POST "$BASE_URL/api/v1/tenants" \
  -H "Content-Type: application/json" \
  -H "X-Tenant-Id: 00000000-0000-0000-0000-000000000000" \
  -H "X-Role: super_admin" \
  -H "X-Principal: admin" \
  -d '{"name":"Test","slug":"test","database_url":"postgresql://postgres:postgres@localhost:5432/tenant_test"}')
TENANT_ID=$(echo $TENANT | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "Tenant: $TENANT_ID"

# 3. Create issuer key
ISSUER_KEY=$(curl -s -X POST "$BASE_URL/api/v1/tenants/$TENANT_ID/api-keys" \
  -H "Content-Type: application/json" \
  -H "X-Tenant-Id: 00000000-0000-0000-0000-000000000000" \
  -H "X-Role: super_admin" \
  -H "X-Principal: admin" \
  -d '{"name":"Issuer","role":"issuer_manager"}')
ISSUER_API_KEY=$(echo $ISSUER_KEY | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)
echo "Issuer Key: $ISSUER_API_KEY"

# 4. Create credential definition
CREDDEF=$(curl -s -X POST "$BASE_URL/api/v1/issuer/credential-definitions" \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: $ISSUER_API_KEY" \
  -d '{"name":"TestCred","format":"jwt_vc_json","schema":{"type":"object","properties":{"test":{"type":"string"}}}}')
CREDDEF_ID=$(echo $CREDDEF | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "CredDef: $CREDDEF_ID"

# 5. Create offer
OFFER=$(curl -s -X POST "$BASE_URL/api/v1/issuer/offers" \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: $ISSUER_API_KEY" \
  -d "{\"credential_definition_id\":\"$CREDDEF_ID\",\"by_reference\":false}")
echo "Offer: $OFFER"
```

## Additional Resources

- OpenID4VCI Specification: https://openid.net/specs/openid-4-verifiable-credential-issuance-1_0.html
- API Documentation: See source code in `src/routes/`
