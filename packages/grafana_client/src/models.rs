use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[serde(rename_all = "camelCase")]
pub struct CreateDataSource {
    pub name: String,
    pub r#type: String,
    pub access: DataSourceAccess,
    pub url: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub basic_auth: Option<bool>,
    pub basic_auth_user: Option<String>,
    pub basic_auth_password: Option<String>,
    pub with_credentials: Option<bool>,
    pub is_default: Option<bool>,
    // pub json_data: Option<HashMap<String, String>>,
    pub secure_json_data: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDataSourceResponse {
    pub id: i32,
    pub name: String,
    pub message: String,
    pub datasource: DataSource,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSource {
    pub id: i32,
    pub org_id: i32,
    pub name: String,
    pub r#type: String,
    pub type_logo_url: String,
    pub access: DataSourceAccess,
    pub url: String,
    pub password: String,
    pub database: String,
    pub user: String,
    pub basic_auth: bool,
    pub basic_auth_user: String,
    pub basic_auth_password: String,
    pub with_credentials: bool,
    pub is_default: bool,
    // pub json_data: HashMap<String, String>,
    pub secure_json_fields: HashMap<String, bool>,
    pub version: i32,
    pub read_only: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DataSourceAccess {
    Proxy,
    Direct,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataSource {
    pub name: String,
    pub r#type: String,
    pub access: DataSourceAccess,
    pub url: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub basic_auth: Option<bool>,
    pub basic_auth_user: Option<String>,
    pub basic_auth_password: Option<String>,
    pub with_credentials: Option<bool>,
    pub is_default: Option<bool>,
    // pub json_data: Option<HashMap<String, String>>,
    pub secure_json_data: Option<HashMap<String, String>>,
    pub version: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataSourceResponse {
    pub id: i32,
    pub name: String,
    pub message: String,
    pub datasource: DataSource,
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
