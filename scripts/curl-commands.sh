# OpenID4VC Backend - Curl Commands Reference
# Complete API testing commands for tenant → API key → issuance → revoke flow

# =============================================================================
# CONFIGURATION
# =============================================================================

# Set your base URL
BASE_URL="http://localhost:3000"

# Authentication options (choose one):
# Option 1: Master API Key (if configured)
MASTER_KEY=""

# Option 2: Development headers (for local testing)
DEV_TENANT_ID=""
DEV_ROLE="super_admin"
DEV_PRINCIPAL="admin"

# Option 3: Generated API Keys (stored after creation)
API_KEY_ISSUER=""      # Created with issuer_manager role
API_KEY_TENANT_ADMIN="" # Created with tenant_admin role

# IDs (stored after creation)
TENANT_ID=""
TENANT_SLUG=""
CREDENTIAL_DEF_ID=""
ISSUANCE_SESSION_ID=""
CREDENTIAL_ID=""

# =============================================================================
# AUTHENTICATION HELPER FUNCTIONS
# =============================================================================

# Using Master API Key
auth_master() {
    echo "-H 'X-Api-Key: $MASTER_KEY'"
}

# Using Generated API Key
auth_apikey() {
    local key="$1"
    echo "-H 'X-Api-Key: $key'"
}

# Using Development Headers
auth_dev() {
    echo "-H 'X-Tenant-Id: $DEV_TENANT_ID' -H 'X-Role: $DEV_ROLE' -H 'X-Principal: $DEV_PRINCIPAL'"
}

# =============================================================================
# STEP 1: HEALTH CHECKS
# =============================================================================

# Health check
curl -X GET "${BASE_URL}/health"

# Readiness check
curl -X GET "${BASE_URL}/ready"

# Metrics (Prometheus format)
curl -X GET "${BASE_URL}/metrics"

# =============================================================================
# STEP 2: TENANT MANAGEMENT (Requires SuperAdmin)
# =============================================================================

# Create tenant
# Required role: super_admin
curl -X POST "${BASE_URL}/api/v1/tenants" \
    -H "Content-Type: application/json" \
    -H "X-Tenant-Id: $DEV_TENANT_ID" \
    -H "X-Role: super_admin" \
    -H "X-Principal: admin" \
    -d '{
        "name": "University of Example",
        "slug": "university-example",
        "database_url": "postgresql://postgres:postgres@localhost:5432/tenant_university"
    }'

# Create tenant (using master key)
curl -X POST "${BASE_URL}/api/v1/tenants" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $MASTER_KEY" \
    -d '{
        "name": "University of Example",
        "slug": "university-example",
        "database_url": "postgresql://postgres:postgres@localhost:5432/tenant_university"
    }'

# =============================================================================
# STEP 3: API KEY MANAGEMENT (Requires SuperAdmin or TenantAdmin)
# =============================================================================

# Create API key with IssuerManager role (for issuance)
# Response includes: {"id": "...", "api_key": "ok_...", ...}
# SAVE THE api_key - it's only returned once!
curl -X POST "${BASE_URL}/api/v1/tenants/${TENANT_ID}/api-keys" \
    -H "Content-Type: application/json" \
    -H "X-Tenant-Id: $DEV_TENANT_ID" \
    -H "X-Role: super_admin" \
    -H "X-Principal: admin" \
    -d '{
        "name": "Issuer Manager Key",
        "role": "issuer_manager"
    }'

# Create API key with TenantAdmin role (for revocation)
curl -X POST "${BASE_URL}/api/v1/tenants/${TENANT_ID}/api-keys" \
    -H "Content-Type: application/json" \
    -H "X-Tenant-Id: $DEV_TENANT_ID" \
    -H "X-Role: super_admin" \
    -H "X-Principal: admin" \
    -d '{
        "name": "Tenant Admin Key",
        "role": "tenant_admin"
    }'

# Create API key with Verifier role
curl -X POST "${BASE_URL}/api/v1/tenants/${TENANT_ID}/api-keys" \
    -H "Content-Type: application/json" \
    -H "X-Tenant-Id: $DEV_TENANT_ID" \
    -H "X-Role: super_admin" \
    -H "X-Principal: admin" \
    -d '{
        "name": "Verifier Key",
        "role": "verifier"
    }'

# Create API key with ReadOnly role
curl -X POST "${BASE_URL}/api/v1/tenants/${TENANT_ID}/api-keys" \
    -H "Content-Type: application/json" \
    -H "X-Tenant-Id: $DEV_TENANT_ID" \
    -H "X-Role: super_admin" \
    -H "X-Principal: admin" \
    -d '{
        "name": "Read Only Key",
        "role": "readonly"
    }'

# =============================================================================
# STEP 4: CREDENTIAL DEFINITIONS (Requires IssuerManager)
# =============================================================================

# Create credential definition with JWT format
curl -X POST "${BASE_URL}/api/v1/issuer/credential-definitions" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_ISSUER" \
    -d '{
        "name": "UniversityDegree",
        "format": "jwt_vc_json",
        "schema": {
            "type": "object",
            "properties": {
                "credentialSubject": {
                    "type": "object",
                    "properties": {
                        "degree": {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string"},
                                "name": {"type": "string"}
                            },
                            "required": ["type", "name"]
                        },
                        "university": {"type": "string"},
                        "graduationDate": {"type": "string", "format": "date"}
                    },
                    "required": ["degree", "university"]
                }
            }
        }
    }'

# Create credential definition with LDP format
curl -X POST "${BASE_URL}/api/v1/issuer/credential-definitions" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_ISSUER" \
    -d '{
        "name": "ProfessionalCertificate",
        "format": "ldp_vc",
        "schema": {
            "type": "object",
            "properties": {
                "credentialSubject": {
                    "type": "object",
                    "properties": {
                        "certification": {
                            "type": "object",
                            "properties": {
                                "title": {"type": "string"},
                                "issuedBy": {"type": "string"},
                                "dateIssued": {"type": "string"}
                            }
                        }
                    }
                }
            }
        }
    }'

# =============================================================================
# STEP 5: CREDENTIAL OFFERS / ISSUANCE SESSIONS (Requires IssuerManager)
# =============================================================================

# Create credential offer (inline)
# Response includes: {"issuance_session_id": "...", "offer_url": "openid-credential-offer://..."}
curl -X POST "${BASE_URL}/api/v1/issuer/offers" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_ISSUER" \
    -d "{
        \"credential_definition_id\": \"$CREDENTIAL_DEF_ID\",
        \"by_reference\": false
    }"

# Create credential offer (by reference - URL only)
curl -X POST "${BASE_URL}/api/v1/issuer/offers" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_ISSUER" \
    -d "{
        \"credential_definition_id\": \"$CREDENTIAL_DEF_ID\",
        \"by_reference\": true
    }"

# =============================================================================
# STEP 6: OID4VCI PROTOCOL ENDPOINTS (Public - No Auth Required)
# =============================================================================

# OAuth Authorization Server Metadata
curl -X GET "${BASE_URL}/oid4vci/${TENANT_SLUG}/.well-known/oauth-authorization-server"

# OpenID Credential Issuer Metadata
curl -X GET "${BASE_URL}/oid4vci/${TENANT_SLUG}/.well-known/openid-credential-issuer"

# Get credential offer by reference
curl -X GET "${BASE_URL}/oid4vci/${TENANT_SLUG}/credential_offer?issuance_session_id=${ISSUANCE_SESSION_ID}"

# Pushed Authorization Request (PAR)
curl -X POST "${BASE_URL}/oid4vci/${TENANT_SLUG}/par" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "response_type=code" \
    -d "client_id=wallet-client-id" \
    -d "code_challenge=challenge123" \
    -d "code_challenge_method=S256" \
    -d "authorization_details=%5B%7B%22type%22%3A%22openid_credential%22%7D%5D"

# Authorization endpoint (browser redirect - GET request)
curl -X GET "${BASE_URL}/oid4vci/${TENANT_SLUG}/authorize?response_type=code&client_id=wallet-client-id&redirect_uri=app%3A%2F%2Fcallback&scope=openid"

# Token endpoint
curl -X POST "${BASE_URL}/oid4vci/${TENANT_SLUG}/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=authorization_code" \
    -d "code=AUTHORIZATION_CODE" \
    -d "redirect_uri=app://callback" \
    -d "client_id=wallet-client-id" \
    -d "code_verifier=verifier123"

# Credential endpoint
curl -X POST "${BASE_URL}/oid4vci/${TENANT_SLUG}/credential" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ACCESS_TOKEN" \
    -d '{
        "format": "jwt_vc_json",
        "types": ["UniversityDegree"],
        "proof": {
            "proof_type": "jwt",
            "jwt": "PROOF_JWT"
        }
    }'

# Notification endpoint
curl -X POST "${BASE_URL}/oid4vci/${TENANT_SLUG}/notification" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ACCESS_TOKEN" \
    -d '{
        "notification_id": "...",
        "event": "credential_accepted"
    }'

# =============================================================================
# STEP 7: VERIFICATION FLOW (Requires Verifier Role)
# =============================================================================

# Create presentation request
curl -X POST "${BASE_URL}/api/v1/verifier/requests" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_VERIFIER" \
    -d '{
        "credential_types": ["UniversityDegree"],
        "claims": ["degree.name", "university"]
    }'

# =============================================================================
# STEP 8: CREDENTIAL REVOCATION (Requires TenantAdmin)
# =============================================================================

# Revoke credential
curl -X POST "${BASE_URL}/api/v1/issuer/issued/${CREDENTIAL_ID}/revoke" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_TENANT_ADMIN" \
    -d '{
        "reason": "Key compromise"
    }'

# Revoke with different reasons
curl -X POST "${BASE_URL}/api/v1/issuer/issued/${CREDENTIAL_ID}/revoke" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_TENANT_ADMIN" \
    -d '{
        "reason": "Lost or stolen device"
    }'

curl -X POST "${BASE_URL}/api/v1/issuer/issued/${CREDENTIAL_ID}/revoke" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_TENANT_ADMIN" \
    -d '{
        "reason": "Affiliation terminated"
    }'

curl -X POST "${BASE_URL}/api/v1/issuer/issued/${CREDENTIAL_ID}/revoke" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_TENANT_ADMIN" \
    -d '{
        "reason": "Administrative revocation"
    }'

# =============================================================================
# STEP 9: KEY MANAGEMENT (Requires TenantAdmin)
# =============================================================================

# Rotate tenant primary key
curl -X POST "${BASE_URL}/api/v1/keys/rotate" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_TENANT_ADMIN"

# =============================================================================
# STEP 10: UTILITY ENDPOINTS
# =============================================================================

# Resolve DID (Public endpoint)
curl -X GET "${BASE_URL}/api/v1/did/resolve?did=did:key:z123456789"

# Subscribe to audit events SSE stream (Requires authentication)
curl -X GET "${BASE_URL}/api/v1/audit/stream" \
    -H "X-Api-Key: $API_KEY_TENANT_ADMIN" \
    -H "Accept: text/event-stream"

# =============================================================================
# COMPLETE FLOW EXAMPLE (Copy-paste ready)
# =============================================================================

# Set variables
BASE_URL="http://localhost:3000"
MASTER_KEY="your-master-api-key-here"

# 1. Create tenant
TENANT_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/v1/tenants" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $MASTER_KEY" \
    -d '{
        "name": "Example University",
        "slug": "example-university",
        "database_url": "postgresql://postgres:postgres@localhost:5432/tenant_example"
    }')
TENANT_ID=$(echo $TENANT_RESPONSE | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "Created tenant: $TENANT_ID"

# 2. Create IssuerManager API key
ISSUER_KEY_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/v1/tenants/${TENANT_ID}/api-keys" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $MASTER_KEY" \
    -d '{
        "name": "Issuer Key",
        "role": "issuer_manager"
    }')
API_KEY_ISSUER=$(echo $ISSUER_KEY_RESPONSE | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)
echo "Created issuer key: $API_KEY_ISSUER"

# 3. Create credential definition
CREDDEF_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/v1/issuer/credential-definitions" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_ISSUER" \
    -d '{
        "name": "TestCredential",
        "format": "jwt_vc_json",
        "schema": {"type": "object", "properties": {"test": {"type": "string"}}}
    }')
CREDENTIAL_DEF_ID=$(echo $CREDDEF_RESPONSE | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "Created credential definition: $CREDENTIAL_DEF_ID"

# 4. Create credential offer
OFFER_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/v1/issuer/offers" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY_ISSUER" \
    -d "{\"credential_definition_id\": \"$CREDENTIAL_DEF_ID\", \"by_reference\": false}")
ISSUANCE_SESSION_ID=$(echo $OFFER_RESPONSE | grep -o '"issuance_session_id":"[^"]*"' | cut -d'"' -f4)
OFFER_URL=$(echo $OFFER_RESPONSE | grep -o '"offer_url":"[^"]*"' | cut -d'"' -f4)
echo "Created offer. Session: $ISSUANCE_SESSION_ID, URL: $OFFER_URL"

# 5. Create TenantAdmin API key
ADMIN_KEY_RESPONSE=$(curl -s -X POST "${BASE_URL}/api/v1/tenants/${TENANT_ID}/api-keys" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $MASTER_KEY" \
    -d '{
        "name": "Admin Key",
        "role": "tenant_admin"
    }')
API_KEY_TENANT_ADMIN=$(echo $ADMIN_KEY_RESPONSE | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)
echo "Created admin key: $API_KEY_TENANT_ADMIN"

# Note: To revoke, you need the credential_id from the issuance flow
# This requires the wallet to complete the OID4VCI protocol

# =============================================================================
# DEBUGGING / TROUBLESHOOTING
# =============================================================================

# Verbose curl (add -v for verbose output)
curl -v -X GET "${BASE_URL}/health"

# Pretty print JSON response
curl -s -X GET "${BASE_URL}/health" | python3 -m json.tool

# Save response to file
curl -s -X GET "${BASE_URL}/health" -o response.json

# Check HTTP status code only
curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/health"

# Follow redirects
curl -L -X GET "${BASE_URL}/oid4vci/example/.well-known/openid-credential-issuer"

# Time the request
curl -w "\nTime: %{time_total}s\n" -X GET "${BASE_URL}/health"
