# Oso policy bootstrap for multi-tenant OpenID4VC backend.

allow(user, action, resource) if
  check_permission(user, action, resource);

check_permission(user, _action, _resource) if
  user.role = "super_admin";

check_permission(user, action, resource) if
  user.role = "tenant_admin" and
  action in ["create", "read", "update", "delete", "issue", "verify", "revoke"] and
  resource.tenant_id = user.tenant_id;

check_permission(user, action, resource) if
  user.role = "issuer_manager" and
  action in ["create", "read", "issue", "revoke"] and
  resource.tenant_id = user.tenant_id;

check_permission(user, action, resource) if
  user.role = "verifier" and
  action in ["create", "read", "verify"] and
  resource.tenant_id = user.tenant_id;

check_permission(user, action, resource) if
  user.role = "readonly" and
  action = "read" and
  resource.tenant_id = user.tenant_id;
