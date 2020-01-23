use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub address1: String,
    pub address2: String,
    pub city: String,
    pub zip_code: String,
    pub state: String,
    pub country: String,
}

#[derive(Debug, Serialize)]
pub struct CreateOrganization {
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrganizationMembership {
    pub login_or_email: String,
    pub role: Role,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrganizationResponse {
    pub org_id: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CreateUser {
    pub name: Option<String>,
    pub email: Option<String>,
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserResponse {
    pub id: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GenericResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct Organization {
    pub id: i32,
    pub name: String,
    pub address: Address,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationMembership {
    pub org_id: i32,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Viewer,
    Editor,
    Admin,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub login: String,
    pub org_id: i32,
    pub is_grafana_admin: bool,
    pub is_disabled: bool,
    pub is_external: bool,
    pub auth_labels: Option<Vec<String>>,
    pub updated_at: DateTime<FixedOffset>,
    pub created_at: DateTime<FixedOffset>,
}
