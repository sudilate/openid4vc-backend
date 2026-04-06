#!/bin/bash
# OpenID4VC Backend - End-to-End Flow Scripts
# Flow: Tenant → API Key → Issuance → Revoke
#
# Usage:
#   1. Set environment variables (see .env.example)
#   2. Run: ./e2e-flow.sh
#   Or run individual steps

set -e

# Configuration
BASE_URL="${OID4VC_BACKEND__SERVER__BASE_URL:-http://localhost:3000}"
SUPER_ADMIN_KEY="${OID4VC_BACKEND__AUTH__MASTER_API_KEY:-}"  # If using master key auth

# Development mode headers (for local testing without JWT)
DEV_TENANT_ID="${DEV_TENANT_ID:-}"
DEV_ROLE="${DEV_ROLE:-super_admin}"
DEV_PRINCIPAL="${DEV_PRINCIPAL:-admin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Storage for IDs across steps
declare -a CREATED_TENANT_IDS
declare -a CREATED_API_KEYS
declare -a CREDENTIAL_DEFINITION_IDS
declare -a ISSUED_CREDENTIAL_IDS

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Build auth headers
build_auth_headers() {
    local api_key="$1"
    
    if [[ -n "$api_key" ]]; then
        # API Key authentication
        echo "-H 'X-Api-Key: $api_key'"
    elif [[ -n "$SUPER_ADMIN_KEY" ]]; then
        # Master API key
        echo "-H 'X-Api-Key: $SUPER_ADMIN_KEY'"
    elif [[ -n "$DEV_TENANT_ID" ]]; then
        # Development mode
        echo "-H 'X-Tenant-Id: $DEV_TENANT_ID' -H 'X-Role: $DEV_ROLE' -H 'X-Principal: $DEV_PRINCIPAL'"
    else
        log_warn "No authentication configured - using dev mode with super_admin"
        echo "-H 'X-Tenant-Id: 00000000-0000-0000-0000-000000000000' -H 'X-Role: super_admin' -H 'X-Principal: admin'"
    fi
}

# ============================================================================
# STEP 1: Health Check
# ============================================================================
step1_health_check() {
    log_info "Step 1: Checking API health..."
    
    response=$(curl -s "${BASE_URL}/health")
    echo "Response: $response"
    
    if echo "$response" | grep -q '"status":"ok"'; then
        log_info "API is healthy"
    else
        log_error "API health check failed"
        return 1
    fi
}

# ============================================================================
# STEP 2: Create Tenant
# ============================================================================
step2_create_tenant() {
    log_info "Step 2: Creating tenant..."
    
    local tenant_name="${1:-Demo Tenant}"
    local tenant_slug="${2:-demo-tenant}"
    local db_url="${3:-postgresql://postgres:postgres@localhost:5432/tenant_demo}"
    
    auth_headers=$(build_auth_headers)
    
    request_body=$(cat <<EOF
{
    "name": "$tenant_name",
    "slug": "$tenant_slug",
    "database_url": "$db_url"
}
EOF
)
    
    log_info "Creating tenant with slug: $tenant_slug"
    
    # Execute curl command
    response=$(eval "curl -s -X POST \
        ${BASE_URL}/api/v1/tenants \
        $auth_headers \
        -H 'Content-Type: application/json' \
        -d '$request_body'")
    
    echo "Response: $response"
    
    # Extract tenant ID
    tenant_id=$(echo "$response" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
    
    if [[ -n "$tenant_id" ]]; then
        log_info "Tenant created with ID: $tenant_id"
        CREATED_TENANT_IDS+=("$tenant_id")
        export CURRENT_TENANT_ID="$tenant_id"
        export CURRENT_TENANT_SLUG="$tenant_slug"
    else
        log_error "Failed to create tenant"
        return 1
    fi
}

# ============================================================================
# STEP 3: Create API Key (Issuer Manager Role)
# ============================================================================
step3_create_api_key() {
    log_info "Step 3: Creating API key with IssuerManager role..."
    
    local tenant_id="${1:-$CURRENT_TENANT_ID}"
    local key_name="${2:-Issuer-Manager-Key}"
    local role="${3:-issuer_manager}"
    
    if [[ -z "$tenant_id" ]]; then
        log_error "No tenant ID provided or available"
        return 1
    fi
    
    auth_headers=$(build_auth_headers)
    
    request_body=$(cat <<EOF
{
    "name": "$key_name",
    "role": "$role"
}
EOF
)
    
    log_info "Creating API key for tenant: $tenant_id"
    
    response=$(eval "curl -s -X POST \
        ${BASE_URL}/api/v1/tenants/$tenant_id/api-keys \
        $auth_headers \
        -H 'Content-Type: application/json' \
        -d '$request_body'")
    
    echo "Response: $response"
    
    # Extract API key and ID
    api_key=$(echo "$response" | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)
    key_id=$(echo "$response" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
    
    if [[ -n "$api_key" ]]; then
        log_info "API Key created: $api_key"
        log_warn "⚠️  SAVE THIS API KEY - it will not be shown again!"
        CREATED_API_KEYS+=("$api_key")
        export CURRENT_API_KEY="$api_key"
        export CURRENT_KEY_ID="$key_id"
    else
        log_error "Failed to create API key"
        return 1
    fi
}

# ============================================================================
# STEP 4: Create Credential Definition
# ============================================================================
step4_create_credential_definition() {
    log_info "Step 4: Creating credential definition..."
    
    local api_key="${1:-$CURRENT_API_KEY}"
    local def_name="${2:-UniversityDegree}"
    local format="${3:-jwt_vc_json}"
    
    if [[ -z "$api_key" ]]; then
        log_error "No API key provided or available"
        return 1
    fi
    
    request_body=$(cat <<EOF
{
    "name": "$def_name",
    "format": "$format",
    "schema": {
        "type": "object",
        "properties": {
            "degree": {
                "type": "object",
                "properties": {
                    "type": {"type": "string"},
                    "name": {"type": "string"}
                }
            },
            "university": {"type": "string"},
            "graduationDate": {"type": "string"}
        },
        "required": ["degree", "university"]
    }
}
EOF
)
    
    log_info "Creating credential definition: $def_name"
    
    response=$(curl -s -X POST \
        "${BASE_URL}/api/v1/issuer/credential-definitions" \
        -H "X-Api-Key: $api_key" \
        -H "Content-Type: application/json" \
        -d "$request_body")
    
    echo "Response: $response"
    
    # Extract credential definition ID
    def_id=$(echo "$response" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
    
    if [[ -n "$def_id" ]]; then
        log_info "Credential definition created with ID: $def_id"
        CREDENTIAL_DEFINITION_IDS+=("$def_id")
        export CURRENT_CREDENTIAL_DEF_ID="$def_id"
    else
        log_error "Failed to create credential definition"
        return 1
    fi
}

# ============================================================================
# STEP 5: Create Credential Offer (Issuance Session)
# ============================================================================
step5_create_offer() {
    log_info "Step 5: Creating credential offer..."
    
    local api_key="${1:-$CURRENT_API_KEY}"
    local cred_def_id="${2:-$CURRENT_CREDENTIAL_DEF_ID}"
    local by_reference="${3:-false}"
    
    if [[ -z "$api_key" ]]; then
        log_error "No API key provided or available"
        return 1
    fi
    
    if [[ -z "$cred_def_id" ]]; then
        log_error "No credential definition ID provided or available"
        return 1
    fi
    
    request_body=$(cat <<EOF
{
    "credential_definition_id": "$cred_def_id",
    "by_reference": $by_reference
}
EOF
)
    
    log_info "Creating credential offer for definition: $cred_def_id"
    
    response=$(curl -s -X POST \
        "${BASE_URL}/api/v1/issuer/offers" \
        -H "X-Api-Key: $api_key" \
        -H "Content-Type: application/json" \
        -d "$request_body")
    
    echo "Response: $response"
    
    # Extract session ID and offer URL
    session_id=$(echo "$response" | grep -o '"issuance_session_id":"[^"]*"' | cut -d'"' -f4)
    offer_url=$(echo "$response" | grep -o '"offer_url":"[^"]*"' | cut -d'"' -f4)
    
    if [[ -n "$session_id" ]]; then
        log_info "Issuance session created with ID: $session_id"
        log_info "Offer URL: $offer_url"
        export CURRENT_SESSION_ID="$session_id"
        export CURRENT_OFFER_URL="$offer_url"
    else
        log_error "Failed to create credential offer"
        return 1
    fi
}

# ============================================================================
# STEP 6: Get OID4VCI Metadata (Public Endpoint)
# ============================================================================
step6_get_metadata() {
    log_info "Step 6: Getting OID4VCI metadata..."
    
    local tenant_slug="${1:-$CURRENT_TENANT_SLUG}"
    
    if [[ -z "$tenant_slug" ]]; then
        log_error "No tenant slug provided or available"
        return 1
    fi
    
    log_info "Getting OAuth authorization server metadata..."
    response=$(curl -s "${BASE_URL}/oid4vci/$tenant_slug/.well-known/oauth-authorization-server")
    echo "OAuth Metadata: $response"
    
    log_info "Getting OpenID credential issuer metadata..."
    response=$(curl -s "${BASE_URL}/oid4vci/$tenant_slug/.well-known/openid-credential-issuer")
    echo "Issuer Metadata: $response"
}

# ============================================================================
# STEP 7: Simulate Wallet Credential Request (OID4VCI Protocol)
# ============================================================================
step7_wallet_request() {
    log_info "Step 7: Simulating wallet credential request flow..."
    
    local tenant_slug="${1:-$CURRENT_TENANT_SLUG}"
    local session_id="${2:-$CURRENT_SESSION_ID}"
    
    if [[ -z "$tenant_slug" ]] || [[ -z "$session_id" ]]; then
        log_error "Missing tenant slug or session ID"
        return 1
    fi
    
    # This is a simplified simulation - in real flow, wallet would:
    # 1. Parse the credential offer
    # 2. Get authorization code
    # 3. Exchange for access token
    # 4. Request credential
    
    log_info "Fetching credential offer details..."
    response=$(curl -s "${BASE_URL}/oid4vci/$tenant_slug/credential_offer?issuance_session_id=$session_id")
    echo "Credential Offer: $response"
    
    # Note: Full wallet flow requires implementing OID4VCI protocol
    log_warn "Full wallet implementation needed for complete issuance flow"
    log_info "For testing, you can use the offer URL in a wallet app: ${BASE_URL}/oid4vci/$tenant_slug/credential_offer?issuance_session_id=$session_id"
}

# ============================================================================
# STEP 8: List Issued Credentials
# ============================================================================
step8_list_credentials() {
    log_info "Step 8: Listing issued credentials..."
    
    local api_key="${1:-$CURRENT_API_KEY}"
    
    if [[ -z "$api_key" ]]; then
        log_error "No API key provided or available"
        return 1
    fi
    
    # Note: The endpoint for listing credentials depends on your implementation
    # This is a placeholder - adjust according to actual API
    log_info "Querying issued credentials..."
    
    # You'll need to implement or find the correct endpoint
    log_warn "Ensure the list credentials endpoint exists and update this function"
}

# ============================================================================
# STEP 9: Create API Key with TenantAdmin Role (for Revocation)
# ============================================================================
step9_create_revoker_api_key() {
    log_info "Step 9: Creating API key with TenantAdmin role for revocation..."
    
    local tenant_id="${1:-$CURRENT_TENANT_ID}"
    local key_name="${2:-Tenant-Admin-Key}"
    
    if [[ -z "$tenant_id" ]]; then
        log_error "No tenant ID provided or available"
        return 1
    fi
    
    # Use super_admin auth to create the key
    auth_headers=$(build_auth_headers)
    
    request_body=$(cat <<EOF
{
    "name": "$key_name",
    "role": "tenant_admin"
}
EOF
)
    
    log_info "Creating TenantAdmin API key for tenant: $tenant_id"
    
    response=$(eval "curl -s -X POST \
        ${BASE_URL}/api/v1/tenants/$tenant_id/api-keys \
        $auth_headers \
        -H 'Content-Type: application/json' \
        -d '$request_body'")
    
    echo "Response: $response"
    
    # Extract API key
    api_key=$(echo "$response" | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)
    
    if [[ -n "$api_key" ]]; then
        log_info "TenantAdmin API Key created: $api_key"
        log_warn "⚠️  SAVE THIS API KEY - it will not be shown again!"
        export CURRENT_REVOKER_API_KEY="$api_key"
    else
        log_error "Failed to create revoker API key"
        return 1
    fi
}

# ============================================================================
# STEP 10: Revoke Credential
# ============================================================================
step10_revoke_credential() {
    log_info "Step 10: Revoking credential..."
    
    local api_key="${1:-$CURRENT_REVOKER_API_KEY}"
    local credential_id="${2:-}"
    local reason="${3:-Key compromise}"
    
    if [[ -z "$api_key" ]]; then
        log_error "No revoker API key provided or available"
        return 1
    fi
    
    if [[ -z "$credential_id" ]]; then
        log_error "No credential ID provided for revocation"
        log_info "Available issued credential IDs: ${ISSUED_CREDENTIAL_IDS[*]}"
        return 1
    fi
    
    request_body=$(cat <<EOF
{
    "reason": "$reason"
}
EOF
)
    
    log_info "Revoking credential: $credential_id"
    
    response=$(curl -s -X POST \
        "${BASE_URL}/api/v1/issuer/issued/$credential_id/revoke" \
        -H "X-Api-Key: $api_key" \
        -H "Content-Type: application/json" \
        -d "$request_body")
    
    echo "Response: $response"
    
    if echo "$response" | grep -q '"status":"revoked"'; then
        log_info "Credential revoked successfully"
    else
        log_error "Failed to revoke credential"
        return 1
    fi
}

# ============================================================================
# RUN ALL STEPS
# ============================================================================
run_all() {
    log_info "Running complete end-to-end flow..."
    echo ""
    
    # Initial health check
    step1_health_check
    echo ""
    sleep 1
    
    # Create tenant
    step2_create_tenant "Test University" "test-university" "postgresql://postgres:postgres@localhost:5432/tenant_test"
    echo ""
    sleep 1
    
    # Create IssuerManager API key
    step3_create_api_key "$CURRENT_TENANT_ID" "Issuance-Key" "issuer_manager"
    echo ""
    sleep 1
    
    # Create credential definition
    step4_create_credential_definition "$CURRENT_API_KEY" "BachelorDegree"
    echo ""
    sleep 1
    
    # Create credential offer
    step5_create_offer "$CURRENT_API_KEY" "$CURRENT_CREDENTIAL_DEF_ID" "false"
    echo ""
    sleep 1
    
    # Get metadata
    step6_get_metadata "$CURRENT_TENANT_SLUG"
    echo ""
    sleep 1
    
    # Wallet simulation (optional)
    # step7_wallet_request "$CURRENT_TENANT_SLUG" "$CURRENT_SESSION_ID"
    # echo ""
    
    # Create TenantAdmin API key for revocation
    step9_create_revoker_api_key "$CURRENT_TENANT_ID" "Revocation-Admin"
    echo ""
    sleep 1
    
    # Note: For actual revocation, we need a credential ID
    # This would come from the issuance flow or credential listing
    log_warn "To complete revocation, provide a credential ID:"
    log_info "  ./e2e-flow.sh revoke <credential_id>"
    
    echo ""
    log_info "Flow completed! Summary:"
    log_info "  Tenant ID: $CURRENT_TENANT_ID"
    log_info "  Tenant Slug: $CURRENT_TENANT_SLUG"
    log_info "  Issuer API Key: $CURRENT_API_KEY"
    log_info "  Revoker API Key: $CURRENT_REVOKER_API_KEY"
    log_info "  Credential Def ID: $CURRENT_CREDENTIAL_DEF_ID"
    log_info "  Session ID: $CURRENT_SESSION_ID"
}

# ============================================================================
# Individual Command Handlers
# ============================================================================
case "${1:-all}" in
    health)
        step1_health_check
        ;;
    tenant)
        step2_create_tenant "$2" "$3" "$4"
        ;;
    apikey)
        step3_create_api_key "$2" "$3" "$4"
        ;;
    creddef)
        step4_create_credential_definition "$2" "$3" "$4"
        ;;
    offer)
        step5_create_offer "$2" "$3" "$4"
        ;;
    metadata)
        step6_get_metadata "$2"
        ;;
    wallet)
        step7_wallet_request "$2" "$3"
        ;;
    revoker-key)
        step9_create_revoker_api_key "$2" "$3"
        ;;
    revoke)
        step10_revoke_credential "$2" "$3" "$4"
        ;;
    all)
        run_all
        ;;
    help|--help|-h)
        cat <<EOF
OpenID4VC Backend - End-to-End Flow Scripts

Usage: ./e2e-flow.sh [command] [args...]

Commands:
  health              - Check API health
  tenant [name] [slug] [db_url]
                     - Create a new tenant
  apikey [tenant_id] [name] [role]
                     - Create API key for tenant
  creddef [api_key] [name] [format]
                     - Create credential definition
  offer [api_key] [cred_def_id] [by_reference]
                     - Create credential offer
  metadata [tenant_slug]
                     - Get OID4VCI metadata
  wallet [tenant_slug] [session_id]
                     - Simulate wallet request
  revoker-key [tenant_id] [name]
                     - Create TenantAdmin API key
  revoke [api_key] [credential_id] [reason]
                     - Revoke a credential
  all                 - Run complete flow (default)
  help                - Show this help

Environment Variables:
  BASE_URL           - API base URL (default: http://localhost:3000)
  SUPER_ADMIN_KEY    - Master API key for super_admin access
  DEV_TENANT_ID      - Tenant ID for dev mode auth
  DEV_ROLE           - Role for dev mode (default: super_admin)
  DEV_PRINCIPAL      - Principal name for dev mode

Examples:
  ./e2e-flow.sh all
  ./e2e-flow.sh tenant "My Org" "my-org"
  ./e2e-flow.sh apikey <tenant_id> "My Key" "issuer_manager"
  ./e2e-flow.sh revoke <revoker_api_key> <credential_id> "Lost device"
EOF
        ;;
    *)
        log_error "Unknown command: $1"
        echo "Run './e2e-flow.sh help' for usage information"
        exit 1
        ;;
esac
