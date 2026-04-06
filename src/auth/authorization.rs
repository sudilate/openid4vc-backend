use std::collections::HashMap;

use anyhow::Context;
use oso::{Oso, PolarClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    SuperAdmin,
    TenantAdmin,
    IssuerManager,
    Verifier,
    ReadOnly,
    ApiClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    Issue,
    Verify,
    Revoke,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    CredentialDefinition,
    IssuedCredential,
    IssuanceSession,
    VerificationSession,
    Webhook,
    Tenant,
}

#[derive(Clone, PolarClass)]
pub struct PolicyUser {
    #[polar(attribute)]
    pub role: String,
    #[polar(attribute)]
    pub tenant_id: String,
}

#[derive(Clone, PolarClass)]
pub struct PolicyResource {
    #[polar(attribute)]
    pub tenant_id: String,
}

#[derive(Debug, Clone)]
pub struct AuthorizationService {
    role_matrix: HashMap<Role, Vec<(Resource, Action)>>,
    policy_file: String,
}

impl AuthorizationService {
    pub fn new() -> anyhow::Result<Self> {
        let mut role_matrix: HashMap<Role, Vec<(Resource, Action)>> = HashMap::new();
        role_matrix.insert(
            Role::TenantAdmin,
            vec![
                (Resource::CredentialDefinition, Action::Create),
                (Resource::CredentialDefinition, Action::Read),
                (Resource::CredentialDefinition, Action::Update),
                (Resource::Tenant, Action::Update),
                (Resource::IssuedCredential, Action::Read),
                (Resource::IssuedCredential, Action::Revoke),
            ],
        );
        role_matrix.insert(
            Role::IssuerManager,
            vec![
                (Resource::IssuanceSession, Action::Create),
                (Resource::IssuanceSession, Action::Read),
                (Resource::IssuedCredential, Action::Issue),
                (Resource::IssuedCredential, Action::Read),
            ],
        );
        role_matrix.insert(
            Role::Verifier,
            vec![
                (Resource::VerificationSession, Action::Create),
                (Resource::VerificationSession, Action::Read),
                (Resource::IssuedCredential, Action::Verify),
            ],
        );
        role_matrix.insert(
            Role::ReadOnly,
            vec![(Resource::IssuedCredential, Action::Read)],
        );
        role_matrix.insert(Role::ApiClient, vec![(Resource::Webhook, Action::Create)]);
        role_matrix.insert(Role::SuperAdmin, vec![]);

        let service = Self {
            role_matrix,
            policy_file: "policy/authorization.polar".to_string(),
        };

        let _ = service
            .evaluate_oso(Role::SuperAdmin, Action::Read, "tenant-a", "tenant-b")
            .context("failed to bootstrap Oso authorization")?;

        Ok(service)
    }

    pub fn is_allowed(&self, role: Role, resource: Resource, action: Action) -> bool {
        if role == Role::SuperAdmin {
            return true;
        }

        let base_allow = self
            .role_matrix
            .get(&role)
            .is_some_and(|pairs| pairs.contains(&(resource, action)));

        if !base_allow {
            return false;
        }

        self.evaluate_oso(role, action, "tenant-scope", "tenant-scope")
            .unwrap_or(false)
    }

    fn evaluate_oso(
        &self,
        role: Role,
        action: Action,
        actor_tenant_id: &str,
        resource_tenant_id: &str,
    ) -> anyhow::Result<bool> {
        let mut oso = Oso::new();
        oso.register_class(PolicyUser::get_polar_class_builder().name("User").build())?;
        oso.register_class(
            PolicyResource::get_polar_class_builder()
                .name("Resource")
                .build(),
        )?;
        oso.load_files(vec![self.policy_file.as_str()])?;

        let user = PolicyUser {
            role: role.as_policy_str().to_string(),
            tenant_id: actor_tenant_id.to_string(),
        };
        let resource = PolicyResource {
            tenant_id: resource_tenant_id.to_string(),
        };

        Ok(oso.is_allowed(user, action.as_policy_str(), resource)?)
    }
}

impl Role {
    pub fn as_policy_str(self) -> &'static str {
        match self {
            Role::SuperAdmin => "super_admin",
            Role::TenantAdmin => "tenant_admin",
            Role::IssuerManager => "issuer_manager",
            Role::Verifier => "verifier",
            Role::ReadOnly => "readonly",
            Role::ApiClient => "api_client",
        }
    }
}

pub fn role_from_str(raw: &str) -> Option<Role> {
    match raw {
        "super_admin" => Some(Role::SuperAdmin),
        "tenant_admin" => Some(Role::TenantAdmin),
        "issuer_manager" => Some(Role::IssuerManager),
        "verifier" => Some(Role::Verifier),
        "readonly" => Some(Role::ReadOnly),
        "api_client" => Some(Role::ApiClient),
        _ => None,
    }
}

impl Action {
    fn as_policy_str(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Read => "read",
            Action::Update => "update",
            Action::Delete => "delete",
            Action::Issue => "issue",
            Action::Verify => "verify",
            Action::Revoke => "revoke",
        }
    }
}
